$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$warnings = [System.Collections.Generic.List[string]]::new()

function Read-Hardware {
    param(
        [string]$Class,
        [string[]]$Properties,
        [string]$Namespace = 'root/cimv2'
    )
    try {
        Get-CimInstance -Namespace $Namespace -ClassName $Class -Property $Properties -OperationTimeoutSec 8 |
            Select-Object -Property $Properties
    }
    catch {
        $warnings.Add("${Class}: $($_.Exception.Message)")
    }
}

$disks = @(Read-Hardware MSFT_PhysicalDisk @('Model', 'FriendlyName', 'FirmwareVersion', 'BusType', 'MediaType', 'Size', 'LogicalSectorSize', 'PhysicalSectorSize', 'HealthStatus') 'root/Microsoft/Windows/Storage')
$fallback = @()
if (!$disks.Count) {
    $fallback = @(Read-Hardware Win32_DiskDrive @('Model', 'FirmwareRevision', 'InterfaceType', 'Size', 'BytesPerSector'))
}

[ordered]@{
    system = @(Read-Hardware Win32_ComputerSystem @('Manufacturer', 'Model', 'PCSystemType', 'PCSystemTypeEx'))
    os = @(Read-Hardware Win32_OperatingSystem @('Caption', 'Version', 'BuildNumber', 'OSArchitecture'))
    board = @(Read-Hardware Win32_BaseBoard @('Manufacturer', 'Product', 'Version'))
    bios = @(Read-Hardware Win32_BIOS @('Manufacturer', 'SMBIOSBIOSVersion', 'SMBIOSMajorVersion', 'SMBIOSMinorVersion'))
    cpu = @(Read-Hardware Win32_Processor @('Name', 'Manufacturer', 'SocketDesignation', 'NumberOfCores', 'NumberOfLogicalProcessors', 'MaxClockSpeed', 'L2CacheSize', 'L3CacheSize'))
    memory = @(Read-Hardware Win32_PhysicalMemory @('Manufacturer', 'PartNumber', 'Capacity', 'SMBIOSMemoryType', 'FormFactor', 'DeviceLocator', 'BankLabel', 'Speed', 'ConfiguredClockSpeed', 'ConfiguredVoltage', 'DataWidth', 'TotalWidth', 'Attributes'))
    memory_arrays = @(Read-Hardware Win32_PhysicalMemoryArray @('Use', 'MemoryDevices', 'MaxCapacityEx'))
    video = @(Read-Hardware Win32_VideoController @('Name', 'DriverVersion', 'VideoProcessor'))
    disks = $disks
    disk_fallback = $fallback
    slots = @(Read-Hardware Win32_SystemSlot @('SlotDesignation', 'CurrentUsage', 'MaxDataWidth'))
    warnings = $warnings
} | ConvertTo-Json -Depth 4 -Compress
