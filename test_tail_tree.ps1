# Test script for tail tree layout - generates YAML layout and test log files
# Usage: .\test_tail_tree.ps1 [files] [groups] [nested]
# Example: .\test_tail_tree.ps1 10 2 $true

param(
    [int]$NumFiles = 10,
    [int]$NumGroups = 2,
    [bool]$Nested = $false
)

$LogDir = "test_logs"
$LayoutFile = "test_tree_layout.yaml"

# Colors for output
function Write-Color {
    param([string]$Text, [string]$Color = "White")
    Write-Host $Text -ForegroundColor $Color
}

Write-Color "Setting up tail tree test with:" -Color Cyan
Write-Color "  Files: $NumFiles" -Color Green
Write-Color "  Groups: $NumGroups" -Color Green
Write-Color "  Nested: $Nested" -Color Green

# Create log directory
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

# Function to create a log file with initial content
function New-LogFile {
    param([int]$Num)
    $File = Join-Path $LogDir "test_$Num.log"
    $Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    @"
[$Timestamp] Starting test log $Num
[$Timestamp] Initial content for testing
"@ | Set-Content -Path $File -Encoding UTF8
}

# Function to generate a group with files
function New-Group {
    param(
        [int]$GroupNum,
        [string]$Indent,
        [int]$FilesPerGroup,
        [int]$StartFile
    )
    
    # Group names based on common log scenarios
    $GroupNames = @("Application Logs", "System Logs", "Service Logs", "Database Logs", 
                    "Network Logs", "Security Logs", "Performance Logs", "Error Logs")
    $Icons = @("📱", "🖥️", "⚙️", "🗄️", "🌐", "🔒", "📊", "❌")
    
    $GroupName = $GroupNames[$GroupNum % $GroupNames.Count]
    $Icon = $Icons[$GroupNum % $Icons.Count]
    $Collapsed = if ($GroupNum -gt 1) { "true" } else { "false" }
    
    $Output = @"
$Indent- name: "$GroupName"
$Indent  icon: "$Icon"
$Indent  collapsed: $Collapsed

"@
    
    # Add nested groups if requested
    if ($Nested -and $GroupNum -eq 0) {
        $Output += @"
$Indent  groups:
$Indent    - name: "Core Services"
$Indent      files:

"@
        # Add half the files to nested group
        $NestedFiles = [math]::Floor($FilesPerGroup / 2)
        for ($j = 0; $j -lt $NestedFiles; $j++) {
            $FileNum = $StartFile + $j
            $FilePath = (Resolve-Path $LogDir).Path + "\test_$FileNum.log"
            # Convert backslashes to forward slashes for YAML compatibility
            $FilePath = $FilePath -replace '\\', '/'
            $Output += @"
$Indent        - path: "$FilePath"
$Indent          name: "Test Log $FileNum"

"@
            New-LogFile -Num $FileNum
        }
        
        $Output += @"
$Indent    - name: "Background Jobs"
$Indent      collapsed: true
$Indent      files:

"@
        # Add remaining files to second nested group
        for ($j = $NestedFiles; $j -lt $FilesPerGroup; $j++) {
            $FileNum = $StartFile + $j
            $FilePath = (Resolve-Path $LogDir).Path + "\test_$FileNum.log"
            # Convert backslashes to forward slashes for YAML compatibility
            $FilePath = $FilePath -replace '\\', '/'
            $Output += @"
$Indent        - path: "$FilePath"
$Indent          name: "Test Log $FileNum"

"@
            New-LogFile -Num $FileNum
        }
    }
    else {
        # Simple flat files
        $Output += @"
$Indent  files:

"@
        for ($j = 0; $j -lt $FilesPerGroup; $j++) {
            $FileNum = $StartFile + $j
            $FilePath = (Resolve-Path $LogDir).Path + "\test_$FileNum.log"
            # Convert backslashes to forward slashes for YAML compatibility
            $FilePath = $FilePath -replace '\\', '/'
            $Output += @"
$Indent    - path: "$FilePath"
$Indent      name: "Test Log $FileNum"

"@
            New-LogFile -Num $FileNum
        }
    }
    
    return $Output
}

# Start building YAML
$YamlContent = @"
name: "Test Tree Layout - $NumGroups groups, $NumFiles files"
version: 1
settings:
  poll_interval_ms: 250
  auto_expand_active: true

groups:

"@

# Calculate files per group
$FilesPerGroup = [math]::Floor($NumFiles / $NumGroups)
$Remainder = $NumFiles % $NumGroups

# Generate groups
$FileCounter = 1
for ($i = 0; $i -lt $NumGroups; $i++) {
    # Distribute remainder files across first groups
    $FilesInGroup = if ($i -lt $Remainder) { $FilesPerGroup + 1 } else { $FilesPerGroup }
    
    $YamlContent += New-Group -GroupNum $i -Indent "  " -FilesPerGroup $FilesInGroup -StartFile $FileCounter
    $FileCounter += $FilesInGroup
}

# Write YAML file
$YamlContent | Set-Content -Path $LayoutFile -Encoding UTF8

Write-Color "`n✓ Created layout file: $LayoutFile" -Color Green
Write-Color "✓ Created $NumFiles log files in $LogDir/" -Color Green

# Function to append random log entries to files
$Script:ShouldStop = $false

function Start-LogAppender {
    $Messages = @(
        "Processing request from client"
        "Database connection established"
        "Cache hit for key"
        "Starting background job"
        "Completed transaction"
        "Warning: High memory usage detected"
        "Error: Connection timeout"
        "Info: Service health check passed"
        "Debug: Query execution time"
        "Metric: Response time"
    )
    
    $Levels = @("INFO", "WARN", "ERROR", "DEBUG")
    
    while (-not $Script:ShouldStop) {
        # Randomly select a few files to update
        $FilesToUpdate = Get-Random -Minimum 1 -Maximum 4
        
        for ($i = 0; $i -lt $FilesToUpdate; $i++) {
            $FileNum = Get-Random -Minimum 1 -Maximum ($NumFiles + 1)
            $File = Join-Path $LogDir "test_$FileNum.log"
            $Level = $Levels | Get-Random
            $Msg = $Messages | Get-Random
            $Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss.fff"
            $Value = Get-Random -Maximum 1000
            
            Add-Content -Path $File -Value "[$Timestamp] [$Level] $Msg $Value" -Encoding UTF8
        }
        
        Start-Sleep -Milliseconds (Get-Random -Minimum 100 -Maximum 1000)
    }
}

# Start log appender in background job
Write-Color "`nStarting log appender in background..." -Color Yellow
$Job = Start-Job -ScriptBlock ${function:Start-LogAppender} -ArgumentList $NumFiles, $LogDir

# Cleanup function
function Stop-LogAppender {
    Write-Color "`nStopping log appender..." -Color Yellow
    $Script:ShouldStop = $true
    Stop-Job -Job $Job -ErrorAction SilentlyContinue
    Remove-Job -Job $Job -Force -ErrorAction SilentlyContinue
}

# Register cleanup on Ctrl+C
$null = Register-EngineEvent -SourceIdentifier PowerShell.Exiting -Action { Stop-LogAppender }

# Launch vis-grep with the layout
Write-Color "`nLaunching vis-grep with tree layout..." -Color Cyan
Write-Color "Press Ctrl+C to stop`n" -Color Cyan

# Build and run vis-grep
try {
    Write-Host "Building vis-grep..." -ForegroundColor Gray
    
    # Try release build first
    $BuildOutput = cargo build --release 2>&1
    if ($LASTEXITCODE -eq 0) {
        $ExePath = ".\target\release\vis-grep.exe"
    }
    else {
        # Fall back to debug build
        cargo build 2>&1 | Out-Null
        $ExePath = ".\target\debug\vis-grep.exe"
    }
    
    if (Test-Path $ExePath) {
        # Run vis-grep with layout file
        & $ExePath --tail-layout $LayoutFile
    }
    else {
        Write-Error "Could not find vis-grep executable"
    }
}
finally {
    # Cleanup
    Stop-LogAppender
}

