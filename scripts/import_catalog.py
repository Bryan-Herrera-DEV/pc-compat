"""Rebuild the offline catalog from public manufacturer documentation."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import date
import json
from pathlib import Path
import re
import subprocess
from tempfile import NamedTemporaryFile
from urllib.parse import urljoin, urlsplit

from bs4 import BeautifulSoup
import requests

ROOT = Path(__file__).resolve().parents[1] / "catalog"
NVIDIA = "https://www.nvidia.com/en-us/geforce/graphics-cards/compare/"
INTEL = "https://cdrdv2-public.intel.com/841923/Intel-Core-Desktop-Boxed-Processors-Comparison-Chart.pdf"
HOSTS = {
    "www.asus.com", "dlcdnets.asus.com", "dlcdnet.asus.com", "www.gigabyte.com", "download.gigabyte.com",
    "www.nvidia.com", "www.amd.com", "rocm.docs.amd.com", "www.intel.com", "cdrdv2-public.intel.com",
    "www.msi.com", "au.msi.com", "www.asrock.com", "download.asrock.com",
    "www.kingston.com", "www.corsair.com", "www.samsung.com",
    "images.samsung.com", "download.semiconductor.samsung.com",
    "us.msi.com", "storage-asset.msi.com", "www.crucial.com", "stage-uk.crucial.com", "documents.sandisk.com",
}
ASUS_GPU = {
    "PRIME-RX9070XT-O16G": "https://www.asus.com/us/motherboards-components/graphics-cards/prime/prime-rx9070xt-o16g/techspec/",
    "PRIME-RTX5070TI-O16G": "https://www.asus.com/ch-en/motherboards-components/graphics-cards/prime/prime-rtx5070ti-o16g/techspec/",
    "DUAL-RTX5060TI-O16G": "https://www.asus.com/rs/motherboards-components/graphics-cards/dual/dual-rtx5060ti-o16g/techspec/",
}
ASUS_GPU.update({
    sku: f"https://www.asus.com/uk/motherboards-components/graphics-cards/{family}/{sku.lower()}/techspec/"
    for family, skus in {
        "prime": (
            "PRIME-RTX5080-O16G",
            "PRIME-RTX5070-O12G",
            "PRIME-RTX5070TI-16G",
            "PRIME-RTX5060TI-O16G",
            "PRIME-RTX5060-O8G",
            "PRIME-RX9070-O16G",
            "PRIME-RX9060XT-O16G",
            "PRIME-RX9060XT-O8G",
        ),
        "dual": (
            "DUAL-RTX5050-O8G",
            "DUAL-RTX5060TI-O8G",
            "DUAL-RTX5060-8G",
            "DUAL-RTX5070-12G",
            "DUAL-RX9060XT-16G",
            "DUAL-RX7600-O8G",
            "DUAL-RTX4060-O8G",
            "DUAL-RTX4060TI-O8G",
            "DUAL-RTX4070S-O12G",
        ),
        "tuf-gaming": (
            "TUF-RTX5080-O16G-GAMING",
            "TUF-RTX5070TI-O16G-GAMING",
            "TUF-RX9070XT-O16G-GAMING",
            "TUF-RX9070-O16G-GAMING",
        ),
    }.items()
    for sku in skus
})
MAX_DOCUMENT_BYTES = 16_000_000


def source(url):
    parts = urlsplit(url)
    if (parts.scheme != "https" or parts.hostname not in HOSTS or parts.port is not None
            or parts.username or parts.password):
        raise ValueError(f"Source outside the official host allowlist: {url}")
    return url


def get_document(url):
    for _ in range(5):
        with requests.get(source(url), allow_redirects=False, stream=True, timeout=(8, 25)) as response:
            if response.is_redirect:
                url = urljoin(url, response.headers["Location"])
                continue
            response.raise_for_status()
            data = bytearray()
            for chunk in response.iter_content(65536):
                data.extend(chunk)
                if len(data) > MAX_DOCUMENT_BYTES:
                    raise ValueError(f"Document exceeds size limit: {url}")
            return bytes(data)
    raise ValueError(f"Too many redirects: {url}")


def get_html(url):
    return BeautifulSoup(get_document(url), "html.parser", from_encoding="utf-8")


def pdf_text(url, *options):
    document = get_document(url)
    if not document.startswith(b"%PDF-"):
        raise ValueError(f"Expected a PDF: {url}")
    return subprocess.run(
        ["pdftotext", "-enc", "UTF-8", "-layout", *options, "-", "-"],
        input=document, capture_output=True, timeout=45, check=True,
    ).stdout.decode("utf-8")


def canonical(text):
    return " ".join(text.replace("™", "").replace("®", "").split())


def product_title(document):
    title = canonical(document.title.get_text(" ", strip=True)) if document.title else ""
    return re.split(r"[|｜]|\s+-\s+", title, maxsplit=1)[0].upper()


def cells(table):
    return [[canonical(cell.get_text(" ", strip=True)) for cell in row.select("th,td")]
            for row in table.select("tr")]


def bios_value(text):
    return re.sub(r"\s+GO$", "", text, flags=re.I).strip().lower()


def asus_cpu(document, format_):
    for table in document.select("table"):
        rows = cells(table)
        if not rows or "Validated since BIOS" not in rows[0]:
            continue
        if format_ == "asus_legacy" and rows[0][:3] == ["CPU", "Validated since PCB", "Validated since BIOS"]:
            for row in rows[1:]:
                if len(row) >= 3 and row[0]:
                    name, separator, specs = row[0].partition("(")
                    if not separator or not re.search(r",|\b(?:rev|ghz|socket)\b", specs, re.I):
                        name, specs = row[0], ""
                    name = re.sub(r"\b(II|Athlon|Phenom)(?=X[2346]\b)", r"\1 ", name, flags=re.I)
                    yield name.strip(), bios_value(row[2]), row[1], specs.rstrip(")")
            return
        if format_ == "asus_nuxt" and "CPU No." in rows[0]:
            header = rows[0]
            for row in rows[1:]:
                if len(row) != len(header):
                    continue
                record = dict(zip(header, row))
                model = record["CPU No."].strip()
                if model:
                    if model.lower().startswith("ultra "):
                        name = "Core " + re.sub(r"^(Ultra \d)-", r"\1 ", model, flags=re.I)
                    elif model.lower().startswith(("ryzen ", "epyc ", "athlon ", "pentium ", "celeron ")):
                        name = model
                    else:
                        name = f'{record["Processor"]} {model}'
                    specs = " · ".join(record[key] for key in ("Frequency", "Wattage", "L3", "Core") if record.get(key))
                    yield name, bios_value(record["Validated since BIOS"]), "", specs
            return
    raise ValueError(f"Unexpected ASUS table format: {format_}")


def cpu_id(name):
    name = canonical(name).lower()
    name = re.sub(r"^(?:amd|intel)\s+", "", name)
    intel = re.search(r"\b(i[3579]-\d{4,5}[a-z]{0,3})\b", name)
    if intel:
        return intel[1]
    ultra = re.search(r"\bcore ultra [579]\s+(\d{3}[a-z]{0,3})( plus)?\b", name)
    if ultra:
        return f"core-ultra-{ultra[1]}" + ("-plus" if ultra[2] else "")
    return re.sub(r"[^a-z0-9]+", "-", name).strip("-")


def read_board(board):
    for key, value in board.items():
        if key.endswith("source"):
            source(value)
    # Some vendors serve their data only to browsers. Keep those specs curated
    # and leave CPU support rows empty until they can be ingested reliably.
    if board["cpu_format"] == "manual":
        return {**{k: v for k, v in board.items() if k != "cpu_format"}, "cpu_support": []}, []
    if board.get("spec_format") == "pdf":
        text = canonical(pdf_text(board["source"], "-f", "1", "-l", "12")).upper()
        if board["model"].upper() not in text:
            raise ValueError(f"Unexpected motherboard manual for {board['model']}: {board['source']}")
    else:
        spec = get_html(board["source"])
        title = product_title(spec)
        if board["model"].upper() != title:
            raise ValueError(f"Unexpected model on spec page for {board['model']}: {title}")
        text = canonical(spec.get_text(" ", strip=True)).upper()
    compact = re.sub(r"SOCKET(775|115[015]|1200|1700|1851)(?=\D|$)", r"LGA\1", re.sub(r"\s+", "", text))
    if (board["socket"] not in compact and not board.get("socket_source")) or f'DDR{board["ddr"]}' not in compact:
        raise ValueError(f"Socket or memory generation missing from {board['source']}")
    support, products = [], {}
    if board["cpu_format"] != "manual":
        document = get_html(board["cpu_source"])
        title = product_title(document)
        if board["model"].upper() != title:
            raise ValueError(f"Unexpected QVL model for {board['model']}: {title}")
        for name, bios, pcb, detail in asus_cpu(document, board["cpu_format"]):
            key = cpu_id(name)
            if not key or not re.search(r"\d", key):
                raise ValueError(f"Unrecognized CPU model: {name}")
            if bios and bios != "all" and not re.fullmatch(r"[0-9A-Za-z.-]{1,16}", bios):
                raise ValueError(f"Unrecognized BIOS value: {bios}")
            support.append({"product_id": key, "bios": bios, "pcb": pcb, "detail": detail or None})
            legacy_package = re.search(r"\bSocket\s*(AM2\+?|AM3\+?|FM2\+?)(?![\w+])", detail, re.I)
            socket = legacy_package[1].upper() if legacy_package else board["socket"]
            power = re.search(r"\b(\d{2,3})\s*W\b", detail, re.I)
            if name.lower().startswith(("core ", "pentium ", "celeron ")):
                name = "Intel " + name
            elif name.lower().startswith(("ryzen ", "athlon ", "epyc ", "phenom ", "sempron ", "opteron ", "fx-")):
                name = "AMD " + name
            item = {
                "id": key, "sku": key if re.fullmatch(r"i[3579]-[\da-z]+", key) else name,
                "name": name, "summary": detail or f'Socket {board["socket"]} · lista de soporte ASUS',
                "source": board["cpu_source"], "category": "cpu", "socket": socket,
                "base_w": int(power[1]) if power else None,
            }
            prior = products.setdefault(key, item)
            if prior["socket"] != item["socket"]:
                raise ValueError(f'Ambiguous CPU package for {name} in {board["model"]}')
            if prior["summary"] != item["summary"]:
                prior["summary"] = "Variantes publicadas en la lista oficial · verifica OPN, stepping y potencia de la unidad"
            if prior["base_w"] != item["base_w"]:
                prior["base_w"] = None
        if len(support) < 12:
            raise ValueError(f"Unexpectedly small support list for {board['model']}: {len(support)}")
    return {**{k: v for k, v in board.items() if k not in {"cpu_format", "spec_format"}}, "cpu_support": support}, list(products.values())


def read_nvidia():
    products = {}
    for table in get_html(NVIDIA).select("table"):
        rows = cells(table)
        if not rows or not any(re.search(r"\b(?:RTX|GTX) \d", name) for name in rows[0][1:]):
            continue
        for index, raw_name in enumerate(rows[0][1:], 1):
            name = re.sub(r"^G\s*eForce\s+", "GeForce ", raw_name, flags=re.I)
            if not re.fullmatch(r"GeForce (?:RTX|GTX) \d{3,4}[A-Za-z0-9 ()+.-]*", name):
                continue
            attrs = {row[0].lower(): row[index] for row in rows[1:] if len(row) == len(rows[0])}
            memory = attrs.get("standard memory config", "")
            bus = next((label + " " + value for label, value in attrs.items()
                        if "pci express" in label and value.lower() not in {"no", "", "-"}), "")
            gen = re.search(r"\b([123456])(?:\.0)?\b", bus)
            key = re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")
            products[key] = {
                "id": key, "sku": name, "name": f"NVIDIA {name}",
                "summary": " · ".join(filter(None, (memory, f"PCIe {gen[1]}.0" if gen else "", "Modelo de referencia"))),
                "source": NVIDIA, "category": "gpu", "reference": True,
                "pcie_gen": int(gen[1]) if gen else None,
            }
    if len(products) < 25:
        raise ValueError(f"Unexpectedly small NVIDIA comparison table: {len(products)}")
    return list(products.values())


def read_intel():
    extracted = pdf_text(INTEL)
    products = {}
    for page, text in enumerate(extracted.split("\f"), 1):
        for line in text.splitlines():
            if not line.strip():
                continue
            fields = re.split(r"\s{2,}", line.strip())
            sku = fields[0]
            socket = next((value for value in fields if re.fullmatch(r"LGA\d{3,4}", value)), None)
            if not socket:
                continue
            year = next((i for i, value in enumerate(fields[:6]) if re.fullmatch(r"20\d{2}", value)), None)
            if year is None or len(fields) <= year + 10:
                continue
            if re.fullmatch(r"i[3579]-\d{4,5}[A-Z]{0,3}", sku):
                name = "Intel Core " + sku
            elif socket == "LGA1851" and re.fullmatch(r"\d{3}[A-Z]{0,3}(?: Plus)?", sku) and fields[year - 1] in {"5", "7", "9"}:
                name = f"Intel Core Ultra {fields[year - 1]} {sku}"
            else:
                continue
            cores, threads, turbo, watts = (fields[year + offset] for offset in (1, 4, 5, 10))
            if not (cores.isdigit() and threads.isdigit() and watts.isdigit()):
                continue
            key = cpu_id(name)
            clock = f" · hasta {turbo} GHz" if re.fullmatch(r"\d\.\d{2}", turbo) else ""
            products[key] = {
                "id": key, "sku": sku, "name": name, "source": f"{INTEL}#page={page}",
                "category": "cpu", "socket": socket, "base_w": int(watts),
                "summary": f"{cores} núcleos · {threads} hilos{clock} · {watts} W de potencia base · {socket}",
            }
    if len(products) < 100 or not {"i5-12400f", "i7-13700k", "core-ultra-265k"} <= products.keys():
        raise ValueError(f"Unexpectedly small Intel boxed CPU table: {len(products)}")
    return list(products.values())


def read_asus_gpu(sku, url):
    document = get_html(url)
    labels = {}
    for element in document.select(".rowTableTitle"):
        label = canonical(element.get_text(" ", strip=True))
        if label:
            value = canonical(element.parent.parent.get_text(" ", strip=True)).removeprefix(label).strip()
            if value:
                labels.setdefault(label, value)
    if labels.get("Model") != sku:
        raise ValueError(f"Unexpected ASUS GPU model: {url}")
    dimensions = re.search(r"(\d+(?:\.\d+)?)\s*[x×]\s*(\d+(?:\.\d+)?)\s*[x×]\s*(\d+(?:\.\d+)?)\s*mm",
                           labels.get("Dimensions", ""), re.I)
    power = re.search(r"\b(\d{3,4})\s*W\b", labels.get("Recommended PSU", ""))
    gen = re.search(r"PCI Express\s+(\d)\.0", labels.get("Bus Standard", ""), re.I)
    lanes = re.search(r"[x×]\s*(1|2|4|8|16)\b", labels.get("Bus Standard", ""), re.I)
    if not (dimensions and power and gen and labels.get("Power Connectors") and labels.get("Graphic Engine")):
        raise ValueError(f"Incomplete ASUS GPU techspec: {url}")
    length, height, thickness = map(float, dimensions.groups())
    return {
        "id": sku.lower(), "sku": sku, "name": f'ASUS {sku.split("-")[0].title()} {labels["Graphic Engine"]}',
        "summary": " · ".join(filter(None, (labels.get("Video Memory"), dimensions[0],
                                           labels.get("Slot"), labels.get("Power Connectors")))),
        "source": url, "category": "gpu", "pcie_gen": int(gen[1]), "psu_w": int(power[1]),
        "pcie_lanes": int(lanes[1]) if lanes else None,
        "length_mm": length, "height_mm": height, "thickness_mm": thickness, "connector": labels["Power Connectors"],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--date", type=date.fromisoformat, required=True, help="Publication date (YYYY-MM-DD)")
    args = parser.parse_args()
    manifest = json.loads((ROOT / "boards.json").read_text(encoding="utf-8"))
    seeds = json.loads((ROOT / "seed-products.json").read_text(encoding="utf-8"))
    with ThreadPoolExecutor(max_workers=5) as pool:
        futures = [pool.submit(read_board, board) for board in manifest]
        nvidia = pool.submit(read_nvidia)
        intel = pool.submit(read_intel)
        asus_gpu = [pool.submit(read_asus_gpu, sku, url) for sku, url in ASUS_GPU.items()]
        boards, products = [], {}
        for future in futures:
            board, entries = future.result()
            boards.append(board)
            for item in entries:
                prior = products.setdefault(item["id"], item)
                if prior["socket"] != item["socket"]:
                    raise ValueError(f'Ambiguous CPU identity: {item["id"]} ({prior["socket"]}, {item["socket"]})')
        extras = [future.result() for future in asus_gpu]
        for item in nvidia.result() + intel.result() + extras + seeds:
            source(item["source"])
            if item["id"] in products and item["category"] == "cpu" and item["socket"] != products[item["id"]]["socket"]:
                raise ValueError(f'Conflicting seed CPU socket: {item["id"]}')
            products[item["id"]] = item
        for index, board in enumerate(boards):
            revision = f" rev. {board['revision']}" if board.get("revision") else ""
            model = board["model"] + revision
            key = re.sub(r"[^a-z0-9]+", "-", (board["manufacturer"] + "-" + model).lower()).strip("-")
            summary = [board["socket"], f'{board["dimms"]} × DDR{board["ddr"]} {board["form"]}', board.get("form_factor")]
            products[f"board-{key}"] = {
                "id": f"board-{key}", "sku": model, "name": f'{board["manufacturer"]} {model}',
                "summary": " · ".join(filter(None, summary)), "source": board["source"],
                "category": "motherboard", "board_index": index,
            }
    output = ROOT / "snapshot.json"
    snapshot = {
        "schema": 1, "published": args.date.isoformat(), "boards": boards,
        "products": sorted(products.values(), key=lambda item: (item["category"], item["name"].casefold(), item["id"])),
    }
    with NamedTemporaryFile(mode="w", encoding="utf-8", newline="\n", dir=ROOT, suffix=".tmp", delete=False) as handle:
        temp = Path(handle.name)
        try:
            handle.write(json.dumps(snapshot, ensure_ascii=False, indent=2) + "\n")
        except BaseException:
            handle.close()
            temp.unlink(missing_ok=True)
            raise
    try:
        temp.replace(output)
    finally:
        temp.unlink(missing_ok=True)
    print(f'{len(products)} products, {len(boards)} board profiles, {sum(len(b["cpu_support"]) for b in boards)} CPU support rows')


if __name__ == "__main__":
    main()
