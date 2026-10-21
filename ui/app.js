const $ = (id) => document.getElementById(id);
const state = { catalog: null, scan: null, section: 'cpu', category: '', selected: null, limit: 24, evaluation: 0, timer: null };
const categories = { cpu: 'Procesadores', memory: 'Memoria RAM', gpu: 'Tarjetas gráficas', storage: 'Almacenamiento', motherboard: 'Placas base' };
const browseOrder = ['memory', 'gpu', 'storage', 'motherboard', 'cpu'];
const verdicts = {
  pass: 'Compatible en las comprobaciones realizadas',
  conditional: 'Viable con condiciones',
  unknown: 'Faltan datos para decidir',
  fail: 'Incompatible con esta configuración',
};
const levels = { pass: 'Verificado', conditional: 'Condición', unknown: 'Por confirmar', fail: 'Bloqueo' };
const sections = { cpu: 'Procesador', gpu: 'Gráficas', memory: 'Memoria', disks: 'Discos', board: 'Placa y BIOS', system: 'Sistema' };
const WMI = 'https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/computer-system-hardware-classes';
const DXGI = 'https://learn.microsoft.com/en-us/windows/win32/api/dxgi/ns-dxgi-dxgi_adapter_desc1';
const installationKey = 'pc-compat-installation-v1';
let installation = readInstallation();
let offeredInstallation = false;

function readInstallation() {
  try {
    const saved = JSON.parse(localStorage.getItem(installationKey));
    return saved && typeof saved === 'object' ? saved : {};
  } catch {
    return {};
  }
}

function saveInstallation(values) {
  installation = { ...values, answered: true };
  try {
    localStorage.setItem(installationKey, JSON.stringify(installation));
  } catch {
    status('Datos disponibles durante esta sesión', 'ready');
  }
}

function openInstallation() {
  const dialog = $('installation-dialog');
  if (dialog.open) return;
  const form = $('installation-form');
  for (const input of form.querySelectorAll('input')) {
    input.value = installation[input.name] ?? '';
  }
  dialog.showModal();
}

function offerInstallation() {
  const inventory = state.scan?.inventory;
  if (!inventory || offeredInstallation || installation.answered) return;
  // A desktop GPU envelope is not useful during first run on a portable system.
  if (inventory.system.some((entry) => entry.PCSystemType === 2 || [2, 8].includes(entry.PCSystemTypeEx))) return;
  offeredInstallation = true;
  openInstallation();
}

function invoke(command, args) {
  if (!window.__TAURI__?.core) return Promise.reject(new Error('Abre PC Compat como aplicación de escritorio.'));
  return window.__TAURI__.core.invoke(command, args);
}

function node(tag, text, className) {
  const element = document.createElement(tag);
  if (text !== undefined) element.textContent = text;
  if (className) element.className = className;
  if (tag === 'button') element.type = 'button';
  return element;
}

function action(text, run, className) {
  const button = node('button', text, className);
  button.addEventListener('click', run);
  return button;
}

function sourceLink(label, url) {
  const error = node('p', '', 'error');
  error.hidden = true;
  error.setAttribute('role', 'alert');
  const button = action(label, async () => {
    error.hidden = true;
    try { await invoke('open_source', { url }); }
    catch (message) {
      error.textContent = String(message);
      error.hidden = false;
    }
  }, 'source');
  const wrapper = node('div');
  wrapper.append(button, error);
  return wrapper;
}

function route() {
  const id = location.hash.slice(1);
  const active = ['inventory', 'catalog', 'coverage'].includes(id) ? id : 'inventory';
  for (const section of document.querySelectorAll('main > section')) section.hidden = section.id !== active;
  for (const link of document.querySelectorAll('nav a')) {
    if (link.hash === '#' + active) link.setAttribute('aria-current', 'page');
    else link.removeAttribute('aria-current');
  }
  window.scrollTo(0, 0);
}

function gib(bytes) { return (Number(bytes) / 1073741824).toLocaleString('es', { maximumFractionDigits: 1 }) + ' GiB'; }
function status(text, mode) {
  $('status').textContent = text;
  $('status').dataset.mode = mode;
}

function tile(code, name, detail, section) {
  const card = section ? action('', () => {
    state.section = section;
    renderHardware(section);
    $('hardware-detail').scrollIntoView({ block: 'nearest' });
  }, 'tile') : node('div', undefined, 'tile');
  if (section) {
    card.dataset.section = section;
    card.setAttribute('aria-pressed', String(state.section === section));
  }
  card.append(node('span', code, 'eyebrow'), node('strong', name), node('small', detail));
  return card;
}

function boardProfile() {
  return state.scan?.profile_index == null ? null : state.catalog?.boards[state.scan.profile_index];
}

const pretty = {
  Manufacturer: 'Fabricante', Name: 'Nombre', Product: 'Modelo', Version: 'Versión o revisión',
  NumberOfCores: 'Núcleos', NumberOfLogicalProcessors: 'Hilos lógicos', SocketDesignation: 'Socket en firmware',
  SMBIOSBIOSVersion: 'Versión de BIOS', SMBIOSMajorVersion: 'SMBIOS mayor', SMBIOSMinorVersion: 'SMBIOS menor',
  MaxClockSpeed: 'Reloj máximo · MHz', L2CacheSize: 'Caché L2 · KiB', L3CacheSize: 'Caché L3 · KiB',
  DeviceLocator: 'Ranura', BankLabel: 'Banco', Capacity: 'Capacidad', PartNumber: 'SKU',
  ConfiguredClockSpeed: 'Velocidad configurada · MT/s', Speed: 'Velocidad reportada · MT/s',
  ConfiguredVoltage: 'Voltaje configurado · mV', SMBIOSMemoryType: 'Tipo DDR', FormFactor: 'Formato',
  Model: 'Modelo', FriendlyName: 'Nombre en Windows', FirmwareVersion: 'Firmware', FirmwareRevision: 'Firmware',
  BusType: 'Bus', MediaType: 'Medio', Size: 'Capacidad física', HealthStatus: 'Salud reportada',
  LogicalSectorSize: 'Sector lógico · B', PhysicalSectorSize: 'Sector físico · B',
  DriverVersion: 'Controlador', VideoProcessor: 'Procesador gráfico',
  name: 'Adaptador', pci_ids: 'Identificadores PCI', dedicated_bytes: 'VRAM dedicada', shared_bytes: 'Memoria compartida',
};
const valueMaps = {
  SMBIOSMemoryType: { 19: 'DDR2', 24: 'DDR3', 26: 'DDR4', 30: 'LPDDR4', 34: 'DDR5', 35: 'LPDDR5' },
  FormFactor: { 8: 'DIMM', 12: 'SO-DIMM' },
  BusType: { 1: 'SCSI', 7: 'USB', 8: 'RAID', 10: 'SAS', 11: 'SATA', 17: 'NVMe' },
  MediaType: { 3: 'HDD', 4: 'SSD', 5: 'SCM' },
  HealthStatus: { 0: 'Saludable', 1: 'Advertencia', 2: 'No saludable' },
};

function formatValue(key, value) {
  if (['Capacity', 'Size', 'dedicated_bytes', 'shared_bytes'].includes(key)) return gib(value);
  if (key === 'MaxCapacityEx') return gib(Number(value) * 1024);
  return valueMaps[key]?.[value] ?? String(value).trim();
}

function hardwareRows(panel, title, rows) {
  for (const [index, row] of (rows ?? []).entries()) {
    const block = node('section', undefined, 'data-block');
    const heading = row.Name ?? row.Product ?? row.DeviceLocator ?? row.Model ?? row.name;
    block.append(node('h3', heading?.trim() || (rows.length > 1 ? title + ' ' + (index + 1) : title)));
    const list = node('dl');
    for (const [key, value] of Object.entries(row)) {
      if (value === null || value === undefined || String(value).trim() === '') continue;
      list.append(node('dt', pretty[key] ?? key), node('dd', formatValue(key, value)));
    }
    block.append(list);
    panel.append(block);
  }
}

function capacityMeters(rows, label, bytes) {
  const known = rows.filter((row) => Number(bytes(row)) > 0);
  const group = node('span', undefined, 'capacity-meters');
  const max = Math.max(...known.map((row) => Number(bytes(row))));
  for (const row of known.slice(0, 4)) {
    const track = node('span', undefined, 'capacity-row');
    const title = node('span', undefined, 'capacity-label');
    const name = label(row);
    title.append(node('span', name), node('span', gib(bytes(row))));
    const meter = node('meter');
    meter.min = 0;
    meter.max = max;
    meter.value = Number(bytes(row));
    meter.setAttribute('aria-hidden', 'true');
    track.append(title, meter);
    group.append(track);
  }
  if (known.length > 4) group.append(node('small', 'Ver ' + (known.length - 4) + ' más en detalles'));
  return group;
}

function renderHardware(focusTile) {
  const data = state.scan?.inventory;
  $('hardware').hidden = !data;
  $('hardware-tiles').hidden = !data;
  if (!data) return;
  const ram = data.memory.reduce((sum, row) => sum + Number(row.Capacity || 0), 0);
  const cpu = data.cpu[0];
  const adapter = data.adapters[0];
  const disks = data.disks.length ? data.disks : data.disk_fallback;
  const diskBytes = disks.reduce((sum, row) => sum + Number(row.Size || 0), 0);
  const ramCard = tile('Memoria RAM', ram ? gib(ram) : 'Sin identificar',
    data.memory.length + (data.memory.length === 1 ? ' módulo detectado' : ' módulos detectados'), 'memory');
  ramCard.append(capacityMeters(data.memory, (row) => row.DeviceLocator || 'Módulo RAM', (row) => row.Capacity));
  const diskCard = tile('Almacenamiento', diskBytes ? gib(diskBytes) : 'Capacidad sin identificar',
    disks.length + (disks.length === 1 ? ' unidad detectada' : ' unidades detectadas'), 'disks');
  diskCard.append(capacityMeters(disks, (row) => row.Model || row.FriendlyName || 'Disco físico', (row) => row.Size));
  $('hardware-tiles').replaceChildren(
    ramCard,
    tile('Procesador', (cpu?.Name ?? 'Sin identificar').replace(/\((R|TM)\)/g, '').split(' @ ')[0].replace(/ CPU$/, ''),
      cpu?.NumberOfCores ? cpu.NumberOfCores + ' núcleos · ' + cpu.NumberOfLogicalProcessors + ' hilos' : 'Ver procesadores', 'cpu'),
    tile('Tarjeta gráfica', adapter?.name ?? data.video[0]?.Name ?? 'Sin identificar',
      adapter?.dedicated_bytes ? gib(adapter.dedicated_bytes) + ' de VRAM dedicada' : 'Ver adaptadores', 'gpu'),
    diskCard,
  );
  $('hardware-tabs').replaceChildren(...Object.entries(sections).map(([key, label]) => {
    const button = action(label, () => {
      state.section = key;
      renderHardware();
      $('hardware-tabs').querySelector('[data-section="' + key + '"]')?.focus();
    });
    button.dataset.section = key;
    button.setAttribute('aria-pressed', String(state.section === key));
    return button;
  }));
  const groups = {
    cpu: [['Procesador', data.cpu]],
    gpu: [['Adaptador DXGI', data.adapters], ['Controlador WMI', data.video]],
    memory: [['Módulo', data.memory], ['Arreglo de memoria', data.memory_arrays]],
    disks: [['Disco físico', data.disks.length ? data.disks : data.disk_fallback]],
    board: [['Placa base', data.board], ['Firmware', data.bios], ['Ranura reportada', data.slots]],
    system: [['Equipo', data.system], ['Sistema operativo', data.os]],
  };
  const panel = $('hardware-detail');
  panel.replaceChildren();
  for (const [title, rows] of groups[state.section]) hardwareRows(panel, title, rows);
  if (!panel.childElementCount) panel.append(node('p', 'Windows no devolvió datos para esta sección.', 'muted'));
  if (state.section === 'board' && boardProfile()) {
    const profile = boardProfile();
    panel.append(sourceLink('Ficha oficial de la placa', profile.source));
    const expansion = boardExpansion(profile);
    if (expansion) panel.append(expansion);
  }
  if (state.section === 'system' && data.warnings.length) {
    panel.append(node('h3', 'Advertencias de detección'), ...data.warnings.map((warning) => node('p', warning, 'muted')));
  }
  if (focusTile) $('hardware-tiles').querySelector('[data-section="' + focusTile + '"]')?.focus();
}

function boardExpansion(profile) {
  if (!profile?.pcie_slots?.length) return null;
  const detail = node('details', undefined, 'data-block');
  detail.append(node('summary', 'Ranuras PCIe según el fabricante'));
  const list = node('ul');
  for (const slot of profile.pcie_slots) {
    const modes = slot.modes.map((mode) => [
      mode.generation && 'Gen' + mode.generation,
      mode.lanes && '×' + mode.lanes + ' eléctrico',
      mode.cpu_families?.length && 'según el CPU',
    ].filter(Boolean).join(' ')).filter(Boolean);
    const item = node('li', [slot.name, 'conector ×' + slot.connector_lanes, modes.join(' / '), slot.note]
      .filter(Boolean).join(' · '));
    list.append(item);
  }
  detail.append(list, sourceLink('Especificaciones de expansión', profile.pcie_source ?? profile.source));
  return detail;
}

async function scan() {
  const rescan = $('rescan');
  if (rescan.disabled) return;
  rescan.disabled = true;
  $('copy').hidden = true;
  $('scan-error').hidden = true;
  state.scan = null;
  renderHardware();
  renderProduct();
  status('Detectando hardware', 'loading');
  $('board-name').textContent = 'Detectando hardware de Windows…';
  $('board-status').textContent = 'Buscando un perfil oficial para la placa detectada.';
  try {
    state.scan = await invoke('scan_hardware');
    renderScanHeader();
    $('copy').hidden = false;
    status('Inventario listo', 'ready');
  } catch (error) {
    $('scan-error').textContent = String(error);
    $('scan-error').hidden = false;
    $('board-name').textContent = 'Inventario pendiente';
    $('board-status').textContent = 'Vuelve a detectar el equipo para evaluar una pieza.';
    status('Error de detección', 'error');
  } finally {
    rescan.disabled = false;
    renderHardware();
    renderResults();
    renderProduct();
    offerInstallation();
  }
}

function renderScanHeader() {
  const profile = boardProfile();
  const board = state.scan?.inventory.board[0];
  const bios = state.scan?.inventory.bios[0]?.SMBIOSBIOSVersion;
  $('board-name').textContent = board?.Product?.trim() || 'Placa sin identificar';
  $('board-status').textContent = [bios && 'BIOS ' + bios, profile ? 'Perfil oficial encontrado para tu placa.' : 'Perfil de placa pendiente. Puedes revisar requisitos físicos y fichas oficiales.'].filter(Boolean).join(' · ');
}

function hasExactGpu(product) {
  return product.category === 'gpu' && !product.reference &&
    Boolean(product.length_mm && product.height_mm && product.thickness_mm && product.psu_w && (product.connector || product.slot_power_only));
}

function normalize(value) {
  return value.normalize('NFD').toLowerCase().replace(/\p{Diacritic}/gu, '').replace(/[^a-z0-9]+/g, ' ').trim();
}

function renderCategories() {
  const products = state.catalog?.products ?? [];
  const counts = new Map(browseOrder.map((category) => [category, 0]));
  for (const product of products) counts.set(product.category, (counts.get(product.category) ?? 0) + 1);
  $('catalog-total').textContent = state.catalog ? products.length.toLocaleString('es') : '—';
  $('catalog-shortcuts').replaceChildren(...browseOrder.map((category) => {
    const button = action(categories[category], () => {
      $('search').value = '';
      setCategory(category);
      location.hash = 'catalog';
    }, 'shortcut');
    button.dataset.category = category;
    button.append(node('span', state.catalog ? String(counts.get(category)) : '—'));
    return button;
  }));
  $('category-tabs').replaceChildren(...['', ...browseOrder].map((category) => {
    const name = categories[category] ?? 'Todas';
    const button = action(name, () => {
      setCategory(category);
      [...$('category-tabs').children].find((tab) => tab.dataset.category === category)?.focus();
    });
    button.dataset.category = category;
    button.setAttribute('aria-pressed', String(state.category === category));
    return button;
  }));
}

function setCategory(category) {
  if (category !== state.category) {
    state.category = category;
    state.limit = 24;
    if (state.selected && category && category !== state.selected.category) {
      state.selected = null;
      renderProduct();
    }
  }
  for (const tab of $('category-tabs').children) {
    tab.setAttribute('aria-pressed', String(tab.dataset.category === category));
  }
  renderResults();
}

function filteredProducts() {
  if (!state.catalog) return [];
  const { category } = state;
  const query = normalize($('search').value);
  const terms = query.split(' ').filter(Boolean);
  const listed = category === 'cpu' && $('listed').checked
    ? new Set((boardProfile()?.cpu_support ?? []).map((item) => item.product_id)) : null;
  const memoryType = category === 'memory' && $('memory-type').value;
  const matches = state.catalog.products.filter((product) => {
    if (category && product.category !== category) return false;
    if (listed && !listed.has(product.id)) return false;
    if (category === 'gpu' && $('exact-gpu').checked && !hasExactGpu(product)) return false;
    if (memoryType && memoryType !== product.form && memoryType !== 'DDR' + product.ddr) return false;
    const text = normalize([product.name, product.sku, product.summary, categories[product.category]].join(' '));
    return terms.every((term) => text.includes(term));
  });
  const exact = matches.filter((product) => normalize(product.sku) === query);
  if (exact.length) return exact;
  if (category === 'gpu') {
    matches.sort((left, right) => Number(hasExactGpu(right)) - Number(hasExactGpu(left)));
  }
  if (!category) {
    const groups = browseOrder.map((kind) => matches.filter((item) => item.category === kind));
    groups[1].sort((left, right) => Number(hasExactGpu(right)) - Number(hasExactGpu(left)));
    const ordered = [];
    const rows = Math.max(...groups.map((group) => group.length));
    for (let i = 0; i < rows; i += 1) {
      for (const group of groups) if (group[i]) ordered.push(group[i]);
    }
    return ordered;
  }
  return matches;
}

function productTag(product) {
  if (product.category === 'gpu') return hasExactGpu(product) ? 'Medidas oficiales' : 'GPU de referencia';
  if (product.category === 'memory') return 'DDR' + product.ddr + ' · ' + product.form;
  if (product.category === 'storage') return product.interface === 'sata_25' ? 'SATA · 2,5″' : 'NVMe · ' + product.interface.slice(5);
  if (product.category === 'motherboard') {
    const board = state.catalog.boards[product.board_index];
    return board.socket + ' · DDR' + board.ddr;
  }
  return product.socket;
}

function renderResults() {
  const canFilter = Boolean(boardProfile()?.cpu_support.length) && state.category === 'cpu';
  $('board-filter').hidden = !canFilter;
  if (!canFilter) $('listed').checked = false;
  $('gpu-filter').hidden = state.category !== 'gpu';
  $('memory-filter').hidden = state.category !== 'memory';
  const matches = filteredProducts();
  $('result-count').textContent = state.catalog ? matches.length + (matches.length === 1 ? ' referencia' : ' referencias') : 'Cargando…';
  const list = $('results');
  list.replaceChildren(...matches.slice(0, state.limit).map((product) => {
    const button = action('', () => selectProduct(product), 'result');
    button.setAttribute('aria-pressed', String(state.selected?.id === product.id));
    button.append(node('span', productTag(product), 'result-tag'), node('strong', product.name), node('small', product.sku));
    return button;
  }));
  if (state.catalog && !matches.length) list.append(node('p', 'Sin coincidencias. Prueba otro nombre, SKU o filtro.', 'muted empty-results'));
  $('more').hidden = matches.length <= state.limit;
}

function selectProduct(product) {
  if (state.selected?.id !== product.id) {
    state.selected = product;
    renderResults();
    renderProduct();
  }
  $('product').focus({ preventScroll: true });
  if (matchMedia('(max-width: 760px)').matches) $('product').scrollIntoView({ block: 'start' });
}

function field(label, input, wide = false) {
  const element = node('label', label, wide ? 'field wide' : 'field');
  element.append(input);
  return element;
}

function choice(form, name, label, values, wide = false) {
  const input = node('select');
  input.name = name;
  for (const [value, title] of values) {
    const option = node('option', title);
    option.value = value;
    input.append(option);
  }
  form.append(field(label, input, wide));
}

function confirm(form, name, label) {
  choice(form, name, label, [['unknown', 'Por revisar'], ['yes', 'Sí'], ['no', 'No']], true);
}

function numberField(form, name, label, step, max) {
  const input = node('input');
  input.type = 'number';
  input.name = name;
  input.min = step === 'any' ? '0.1' : step;
  input.max = max;
  input.step = step;
  if (typeof installation[name] === 'number' && Number.isFinite(installation[name])) {
    input.value = installation[name];
  }
  form.append(field(label, input));
}

function productForm(product) {
  const panel = node('section', undefined, 'form');
  panel.append(node('h3', 'Datos de la instalación'));
  const form = node('form', undefined, 'form-grid');
  form.addEventListener('submit', (event) => event.preventDefault());
  if (product.category === 'cpu' && boardProfile()) {
    confirm(form, 'cooler', 'Disipador, montaje y alimentación adecuados');
  }
  if (product.category === 'memory') {
    choice(form, 'replace_memory', 'Plan de memoria', [['add', 'Añadir al equipo'], ['replace', 'Reemplazar toda la RAM']]);
    if (boardProfile()) confirm(form, 'memory_qvl', 'SKU, revisión y población confirmados en la QVL');
  }
  if (product.category === 'gpu') {
    const slots = boardProfile()?.pcie_slots ?? [];
    if (slots.length) choice(form, 'pcie_slot', 'Ranura según la ficha de tu placa',
      slots.map((slot) => [slot.name, slot.name + ' · conector ×' + slot.connector_lanes]), true);
    confirm(form, 'gpu_slot', slots.length ? 'La ranura queda libre y hay espacio adyacente' : 'Ranura PCIe y espacio adyacente disponibles');
    if (product.connector && !product.slot_power_only) confirm(form, 'gpu_cable', 'Cableado de alimentación adecuado para este SKU');
    if (product.psu_w) numberField(form, 'psu_w', 'Potencia de la fuente (W)', 1, 65535);
    if (product.length_mm) numberField(form, 'gpu_length', 'Largo libre (mm)', 'any', 3000);
    if (product.height_mm) numberField(form, 'gpu_height', 'Alto libre (mm)', 'any', 3000);
    if (product.thickness_mm) numberField(form, 'gpu_thickness', 'Grosor libre (mm)', 'any', 3000);
  }
  if (product.category === 'storage') {
    const size = product.interface.startsWith('nvme_') ? product.interface.slice(5) : null;
    const slots = size ? boardProfile()?.m2_slots ?? [] : [];
    if (slots.length) choice(form, 'm2_slot', 'Ranura de destino', [['', 'Elegir ranura'], ...slots.map((slot) => [slot.name, slot.name])]);
    if (!size || !boardProfile()?.m2_known_absent) {
      confirm(form, 'storage_install', product.interface === 'sata_25' ? 'Bahía, puerto SATA y cables disponibles' : 'Ranura NVMe ' + size + ' libre y con espacio para el montaje');
    }
  }
  if (product.category === 'motherboard') confirm(form, 'board_fit', 'Gabinete y conectores de la fuente compatibles con esta placa');
  form.addEventListener('input', (event) => {
    if (event.target.name === 'm2_slot') form.elements.storage_install.value = 'unknown';
    if (event.target.name === 'pcie_slot') form.elements.gpu_slot.value = 'unknown';
    if (event.target.name === 'replace_memory' && form.elements.memory_qvl) form.elements.memory_qvl.value = 'unknown';
    scheduleEvaluation(product, form);
  });
  panel.append(form);
  panel.hidden = !form.childElementCount;
  return [panel, form];
}

function collectInputs(form) {
  const inputs = {};
  for (const [key, value] of new FormData(form)) {
    const input = form.elements[key];
    if (key === 'replace_memory') inputs[key] = value === 'replace';
    else if (input.type === 'number') {
      const number = input.valueAsNumber;
      if (input.validity.valid && Number.isFinite(number) && number > 0) inputs[key] = number;
    } else if (value) inputs[key] = value;
  }
  return inputs;
}

function scheduleEvaluation(product, form, delay = 110) {
  const generation = ++state.evaluation;
  clearTimeout(state.timer);
  state.timer = setTimeout(async () => {
    try {
      const report = await invoke('evaluate_upgrade', { id: product.id, inputs: collectInputs(form) });
      if (generation === state.evaluation) renderReport(report);
    } catch (error) {
      if (generation === state.evaluation) renderReport({ level: 'unknown', checks: [] }, String(error));
    }
  }, delay);
}

function renderReport(report, message) {
  const banner = $('report-banner');
  if (!banner) return;
  banner.dataset.level = report.level;
  banner.replaceChildren(node('small', 'DICTAMEN', 'eyebrow'), node('strong', message ?? verdicts[report.level]));
  $('checks').replaceChildren(...report.checks.map((check) => {
    const row = node('section', undefined, 'check');
    row.dataset.level = check.level;
    row.append(node('h3', levels[check.level] + ' · ' + check.title), node('p', check.detail));
    if (check.source) row.append(sourceLink('Ver evidencia oficial', check.source));
    return row;
  }));
}

function renderProduct() {
  ++state.evaluation;
  clearTimeout(state.timer);
  const panel = $('product');
  const product = state.selected;
  panel.replaceChildren();
  if (!product) {
    panel.append(node('p', 'DICTAMEN', 'eyebrow'), node('h2', 'Selecciona una pieza'),
      node('p', 'Abre una referencia para ver sus requisitos, la evaluación y las fuentes oficiales.', 'muted'));
    return;
  }
  panel.append(node('p', categories[product.category], 'eyebrow'), node('h2', product.name), node('p', product.sku, 'sku muted'),
    node('p', product.summary, 'summary muted'), sourceLink('Ficha oficial', product.source));
  const banner = node('div', undefined, 'report-banner');
  banner.id = 'report-banner';
  banner.setAttribute('role', 'status');
  const checks = node('div');
  checks.id = 'checks';
  const [formPanel, form] = productForm(product);
  panel.append(banner, formPanel, checks);
  if (product.category === 'motherboard') {
    const expansion = boardExpansion(state.catalog.boards[product.board_index]);
    if (expansion) panel.insertBefore(expansion, checks);
  }
  if (!state.scan) formPanel.hidden = true;
  renderReport({ level: 'unknown', checks: [] }, state.scan ? 'Calculando…' : 'Primero detecta el equipo desde Mi equipo.');
  if (state.scan) scheduleEvaluation(product, form, 0);
}

function renderCoverage() {
  const { products, boards, published } = state.catalog;
  const links = boards.reduce((sum, board) => sum + new Set(board.cpu_support.map((entry) => entry.product_id)).size, 0);
  $('coverage-stats').replaceChildren(
    tile('PRODUCTOS', String(products.length), 'Modelos y SKU en el catálogo'),
    tile('PLACAS', String(boards.length), 'Perfiles de fabricante y revisión'),
    tile('CPU × PLACA', String(links), 'Cruces con fila oficial'),
    tile('VERSIÓN', published, 'Fecha de la instantánea local'),
  );
  $('catalog-version').textContent = 'CATÁLOGO ' + published;
  $('coverage-boards').replaceChildren(...boards.map((board) => {
    const row = node('div', undefined, 'coverage-row');
    const description = node('div');
    description.append(node('strong', board.manufacturer + ' ' + board.model + (board.revision ? ' rev. ' + board.revision : '')),
      node('small', board.socket + ' · DDR' + board.ddr + ' · ' + board.cpu_support.length + ' filas de CPU'));
    const links = node('div');
    links.append(sourceLink('Especificaciones', board.source), sourceLink('Lista oficial de CPU', board.cpu_source));
    row.append(description, links);
    return row;
  }));
}

async function loadCatalog() {
  try {
    const payload = await invoke('get_catalog');
    const snapshot = payload?.schema ? payload : JSON.parse(new TextDecoder().decode(new Uint8Array(payload)));
    if (snapshot.schema !== 1) throw new Error('Versión de catálogo no admitida.');
    state.catalog = snapshot;
    renderCategories();
    renderResults();
    renderCoverage();
    if (state.scan) renderScanHeader();
  } catch (error) {
    $('result-count').textContent = 'CATÁLOGO NO DISPONIBLE';
    $('results').append(node('p', String(error), 'error'));
  }
}

$('rescan').addEventListener('click', scan);
$('installation-settings').addEventListener('click', openInstallation);
$('installation-form').addEventListener('submit', (event) => {
  event.preventDefault();
  saveInstallation(collectInputs(event.currentTarget));
  $('installation-dialog').close();
  renderProduct();
});
$('installation-skip').addEventListener('click', () => {
  saveInstallation(installation);
  $('installation-dialog').close();
});
$('installation-dialog').addEventListener('cancel', () => saveInstallation(installation));
$('copy').addEventListener('click', async () => {
  if (!state.scan) return;
  try {
    await navigator.clipboard.writeText(JSON.stringify(state.scan.inventory, null, 2));
    status('Inventario copiado', 'ready');
  } catch {
    status('Portapapeles no disponible', 'error');
  }
});
function resetResults() { state.limit = 24; renderResults(); }
$('search').addEventListener('input', resetResults);
for (const id of ['listed', 'exact-gpu', 'memory-type']) $(id).addEventListener('change', resetResults);
$('more').addEventListener('click', () => { state.limit += 24; renderResults(); });
$('inventory-sources').append(sourceLink('Microsoft CIM/WMI', WMI), sourceLink('Microsoft DXGI', DXGI));
window.addEventListener('hashchange', route);
route();
renderCategories();
renderProduct();
loadCatalog();
scan();
