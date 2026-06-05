#requires -version 3.0
{# template engine Tera #}
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
{% if (assets | length > 0) %}
$OrigDir         = $PWD.Path
$Force           = {{ force | escape_ps1 }}
$App             = {{ app | escape_ps1 }}
$DebugPreference = if ($env:GETPIPE_LOG_LEVEL -eq 'DEBUG') { 'Continue' } else { 'SilentlyContinue' }

#------------------------------------------------------------------------------
# 01) Matching Artifacts
#------------------------------------------------------------------------------
$Urls = @({% for asset in assets %}{{ asset.url | escape_ps1 }}{% if not loop.last %}, {% endif %}{% endfor %})

{% raw %}
#------------------------------------------------------------------------------
# 02) Terminal Cursor Primitives
#------------------------------------------------------------------------------
$Esc = [char]27
function Move-CursorUp      { param([int]$n = 1); [Console]::Error.Write("$Esc[$($n)A") }
function Move-CursorDown    { param([int]$n = 1); [Console]::Error.Write("$Esc[$($n)B") }
function Move-CursorForward { param([int]$n = 1); [Console]::Error.Write("$Esc[$($n)C") }
function Move-CursorBack    { param([int]$n = 1); [Console]::Error.Write("$Esc[$($n)D") }
function Set-CursorLineStart { [Console]::Error.Write("$Esc[1G")   }
function Save-CursorPosition    { [Console]::Error.Write("$Esc[s")    }
function Restore-CursorPosition { [Console]::Error.Write("$Esc[u")    }
function Hide-Cursor        { [Console]::Error.Write("$Esc[?25l") }
function Show-Cursor        { [Console]::Error.Write("$Esc[?25h") }
function Clear-ConsoleLine  { [Console]::Error.Write("$Esc[2K")   }
function Set-StyleBold      { [Console]::Error.Write("$Esc[1m") }
function Set-StyleNormal    { [Console]::Error.Write("$Esc[0m") }
function Set-StyleReverse   { [Console]::Error.Write("$Esc[7m") }

#------------------------------------------------------------------------------
# 03) Multi-Select Menu
#------------------------------------------------------------------------------
function Show-Menu {
    param([bool]$Single, [int]$Current, [string[]]$Items)
    $n     = $Items.Count
    $width = "$n".Length
    for ($i = 0; $i -lt $n; $i++) {
        Set-CursorLineStart; Clear-ConsoleLine
        [Console]::Error.Write("  ")
        $label = ("{0,$width}" -f ($i + 1)) + ")"
        if ($i -eq $Current) { Set-StyleReverse; [Console]::Error.Write($label); Set-StyleNormal }
        else                  { [Console]::Error.Write($label) }
        [Console]::Error.Write(" ")
        if ($script:MenuSelected[$i] -eq 1) { Set-StyleReverse; [Console]::Error.Write($Items[$i]); Set-StyleNormal }
        else                                 { [Console]::Error.Write($Items[$i]) }
        [Console]::Error.Write("`n")
    }
    if (-not $Single) {
        Set-CursorLineStart; Clear-ConsoleLine; [Console]::Error.Write("  ")
        if ($Current -eq $n) { Set-StyleReverse; [Console]::Error.Write("a)"); Set-StyleNormal } else { [Console]::Error.Write("a)") }
        [Console]::Error.Write(" all`n")
    }
    $qIdx = if ($Single) { $n } else { $n + 1 }
    Set-CursorLineStart; Clear-ConsoleLine; [Console]::Error.Write("  ")
    if ($Current -eq $qIdx) { Set-StyleReverse; [Console]::Error.Write("q)"); Set-StyleNormal } else { [Console]::Error.Write("q)") }
    [Console]::Error.Write(" quit`n")
}

function Invoke-MultiSelect {
    param([switch]$Single, [string[]]$Items)
    $n       = $Items.Count
    $extra   = if ($Single) { 1 } else { 2 }
    $current = 0
    $script:MenuSelected = @(0) * $n

    Hide-Cursor
    Show-Menu -Single ([bool]$Single) -Current $current -Items $Items

    while ($true) {
        $key  = [Console]::ReadKey($true)
        $ch   = $key.KeyChar
        $onA  = (-not $Single) -and ($current -eq $n)
        $onQ  = ($Single -and $current -eq $n) -or ((-not $Single) -and $current -eq ($n + 1))

        if ($key.Key -eq [ConsoleKey]::UpArrow) {
            if ($current -gt 0) { $current-- }
        } elseif ($key.Key -eq [ConsoleKey]::DownArrow) {
            if ($current -lt ($n + $extra - 1)) { $current++ }
        } elseif ($ch -ge '1' -and $ch -le '9') {
            $idx = [int][string]$ch - 1
            if ($idx -lt $n) { $current = $idx }
        } elseif ($key.Key -eq [ConsoleKey]::Spacebar) {
            if ($onQ) {
                Show-Cursor; return $null
            } elseif ($onA) {
                $v = if ($script:MenuSelected -notcontains 0) { 0 } else { 1 }
                for ($i = 0; $i -lt $n; $i++) { $script:MenuSelected[$i] = $v }
            } elseif ($Single) {
                $script:MenuSelected[$current] = 1
                Move-CursorUp ($n + $extra)
                Show-Menu -Single ([bool]$Single) -Current $current -Items $Items
                Show-Cursor; return $current
            } else {
                $script:MenuSelected[$current] = if ($script:MenuSelected[$current] -eq 1) { 0 } else { 1 }
            }
        } elseif ($key.Key -eq [ConsoleKey]::Enter) {
            if ($onQ) { Show-Cursor; return $null }
            if ($Single) {
                $script:MenuSelected[$current] = 1
            } elseif ($onA) {
                for ($i = 0; $i -lt $n; $i++) { $script:MenuSelected[$i] = 1 }
            } else {
                if (-not ($script:MenuSelected -contains 1) -and $current -lt $n) {
                    $script:MenuSelected[$current] = 1
                }
            }
            Move-CursorUp ($n + $extra)
            Show-Menu -Single ([bool]$Single) -Current $current -Items $Items
            Show-Cursor
            if ($Single) { return $current }
            return @(0..($n - 1) | Where-Object { $script:MenuSelected[$_] -eq 1 })
        } elseif ($ch -eq 'a' -or $ch -eq 'A') {
            if (-not $Single) {
                $v = if ($script:MenuSelected -notcontains 0) { 0 } else { 1 }
                for ($i = 0; $i -lt $n; $i++) { $script:MenuSelected[$i] = $v }
            }
        } elseif ($ch -eq 'q' -or $ch -eq 'Q') {
            Show-Cursor; return $null
        }

        Move-CursorUp ($n + $extra)
        Show-Menu -Single ([bool]$Single) -Current $current -Items $Items
    }
}

function Invoke-MultiSelectNumbered {
    param([switch]$Single, [string[]]$Items)
    $n = $Items.Count
    for ($i = 0; $i -lt $n; $i++) { Write-Host "  $($i + 1)) $($Items[$i])" }
    if (-not $Single) { Write-Host "  a) all" }
    Write-Host "  q) quit"
    do {
        $prompt = if ($Single) { "Enter choice" } else { "Enter choices (space-separated)" }
        $raw = (Read-Host $prompt).Trim()
        if ($raw -eq 'q' -or $raw -eq 'Q') { return $null }
        if (-not $Single -and ($raw -eq 'a' -or $raw -eq 'A')) { return @(0..($n - 1)) }
        $valid = $true; $indices = @()
        foreach ($token in ($raw -split '\s+')) {
            $num = 0
            if ([int]::TryParse($token, [ref]$num) -and $num -ge 1 -and $num -le $n) {
                $indices += $num - 1
                if ($Single) { break }
            } else { $valid = $false; break }
        }
        if ($valid -and $indices.Count -gt 0) { return @($indices) }
        Write-Host "Invalid input. Try again."
    } while ($true)
}

#------------------------------------------------------------------------------
# 05) Download Helper
#------------------------------------------------------------------------------
function Get-RemoteFile {
    param([string]$Url, [string]$OutFile = "")
    try {
        if ($OutFile) {
            Invoke-WebRequest -Uri $Url -OutFile $OutFile -UseBasicParsing -ErrorAction Stop
            return $true
        }
        return (Invoke-WebRequest -Uri $Url -UseBasicParsing -ErrorAction Stop).Content
    } catch {
        Write-Host "Failed to download from ${Url}: $_"
        if ($OutFile) { return $false } else { return $null }
    }
}

#------------------------------------------------------------------------------
# 06) Overwrite Guard
#------------------------------------------------------------------------------
# Returns $false if the user declines; caller should return/continue.
function Confirm-Overwrite {
    param([string]$Dest)
    if ((Test-Path $Dest) -and ($Force -ne 'true')) {
        $answer = Read-Host "$Dest already exists. Overwrite? [y/N]"
        if ($answer -notmatch '^[yY]') {
            Write-Host "skipping installation"
            return $false
        }
    }
    return $true
}

{% endraw %}
#------------------------------------------------------------------------------
# 07) Installation Prefix
#------------------------------------------------------------------------------
# Helper: select and install executables found in the current directory after
# archive extraction. Returns $false if the user cancelled the selection.
function Install-ArchiveContents {
    param([string]$InstallPrefix, [bool]$IsInteractive)

    $ExeFiles = @(Get-ChildItem -Recurse -File -Filter "*.exe" | Select-Object -ExpandProperty FullName)
    if ($ExeFiles.Count -gt 0) {
        $ExecutableFiles = $ExeFiles
    } else {
        $Candidates = @(Get-ChildItem -Recurse -File | Where-Object { $_.Extension -eq "" } | Select-Object -ExpandProperty FullName)
        if ($App) {
            $Named = @($Candidates | Where-Object { (Split-Path $_ -Leaf) -eq $App })
            $ExecutableFiles = if ($Named.Count -gt 0) { $Named } else { $Candidates }
        } else {
            $ExecutableFiles = $Candidates
        }
    }

    foreach ($f in $ExecutableFiles) { Write-Debug "found: $f" }

    if ($ExecutableFiles.Count -eq 0) {
        throw "no executable files found in archive"
    } elseif ($ExecutableFiles.Count -eq 1) {
        $SelectedIndices = @(0)
    } else {
        Write-Host "Select binaries to install:"
        $Result = if ($IsInteractive) { Invoke-MultiSelect -Items $ExecutableFiles }
                  else                { Invoke-MultiSelectNumbered -Items $ExecutableFiles }
        if ($null -eq $Result) { return $false }
        $SelectedIndices = @($Result)
    }

    $DefaultDir = Join-Path $InstallPrefix "bin"
    $InstallDir = (Read-Host "Install directory [$DefaultDir]").Trim()
    if (-not $InstallDir) { $InstallDir = $DefaultDir }
    if (-not (Test-Path $InstallDir)) { New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null }

    foreach ($i in $SelectedIndices) {
        $SrcFile     = $ExecutableFiles[$i]
        $SrcName     = Split-Path $SrcFile -Leaf
        $DefaultName = if ($App) { Split-Path $App -Leaf } else { $SrcName }
        $DestName    = (Read-Host "Install '$SrcName' as [$DefaultName]").Trim()
        if (-not $DestName) { $DestName = $DefaultName }
        $DestPath = Join-Path $InstallDir $DestName
        if (-not (Confirm-Overwrite $DestPath)) { continue }
        Copy-Item $SrcFile $DestPath -Force
        Write-Host "Installed $DestName to $DestPath"
        Write-Debug "installed: $SrcFile -> $DestPath"
    }
    return $true
}


{% if prefix and prefix != "auto" %}
{% else %}
function Get-InstallPrefix {
    $IsAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    if ($IsAdmin) { return $env:ProgramFiles }
    $LocalPrograms = Join-Path $env:LOCALAPPDATA "Programs"
    if (Test-Path $LocalPrograms) { return $LocalPrograms }
    return $OrigDir
}
{% endif %}

#------------------------------------------------------------------------------
# Main
#------------------------------------------------------------------------------
function Invoke-PipeInstall {
    #--------------------------------------------------------------------------
    # 08) Temporary Workspace and Exit Cleanup
    #--------------------------------------------------------------------------
    $TmpDir = [IO.Path]::GetTempFileName()
    Remove-Item $TmpDir -Force
    New-Item -ItemType Directory -Path $TmpDir | Out-Null
    Write-Debug "workdir: $TmpDir"
    Push-Location $TmpDir
    try {
        #----------------------------------------------------------------------
        # 07) Installation Prefix (continued)
        #----------------------------------------------------------------------
{% if prefix and prefix != "auto" %}
        $InstallPrefix = {{ prefix | escape_ps1 }}
{% else %}
        $InstallPrefix = Get-InstallPrefix
{% endif %}
        Write-Debug "prefix: $InstallPrefix"

        #----------------------------------------------------------------------
        # 09) Asset Arrays
        #----------------------------------------------------------------------
        $Filenames  = @({% for asset in assets %}{{ asset.name | escape_ps1 }}{% if not loop.last %}, {% endif %}{% endfor %})
        $Filetypes  = @({% for asset in assets %}{{ asset.filetype | escape_ps1 }}{% if not loop.last %}, {% endif %}{% endfor %})
        $Printables = @({% for asset in assets %}{{ asset.name ~ " (" ~ asset.filetype ~ ")" | escape_ps1 }}{% if not loop.last %}, {% endif %}{% endfor %})

        #----------------------------------------------------------------------
        # 10) Asset Selection
        #----------------------------------------------------------------------
        Write-Host "Please select one of the following:"
        $IsInteractive = -not [Console]::IsInputRedirected -and -not [Console]::IsErrorRedirected
        $Choice = if ($IsInteractive) { Invoke-MultiSelect -Single -Items $Printables }
                  else                { Invoke-MultiSelectNumbered -Single -Items $Printables }
        if ($null -eq $Choice) { return }
        Write-Debug "selected: $Choice"

        #----------------------------------------------------------------------
        # 11) Download and Install Dispatch
        #----------------------------------------------------------------------
        $FileType = $Filetypes[$Choice]
        Write-Debug "artifact: $($Urls[$Choice]), type: $FileType"
        Write-Host "Downloading from $($Urls[$Choice]) to $TmpDir"

        switch ($FileType) {
            "binary" {
                $Filename   = $Filenames[$Choice]
                $SavedFile  = Join-Path $TmpDir $Filename
                if (-not (Get-RemoteFile $Urls[$Choice] $SavedFile)) {
                    throw "failed downloading binary asset"
                }
                Write-Debug "downloaded: $SavedFile"

                $DefaultName = if ($App) { Split-Path $App -Leaf } else { $Filename }
                $BinaryName  = (Read-Host "Binary name [$DefaultName]").Trim()
                if (-not $BinaryName) { $BinaryName = $DefaultName }

                $DefaultDir = Join-Path $InstallPrefix "bin"
                $BinaryDir  = (Read-Host "Install directory [$DefaultDir]").Trim()
                if (-not $BinaryDir) { $BinaryDir = $DefaultDir }

                if (-not (Test-Path $BinaryDir)) { New-Item -ItemType Directory -Path $BinaryDir -Force | Out-Null }
                $DestPath = Join-Path $BinaryDir $BinaryName
                if (-not (Confirm-Overwrite $DestPath)) { return }
                Copy-Item $SavedFile $DestPath -Force
                Write-Host "Installed $BinaryName to $DestPath"
            }
            "deb installer" {
                throw "deb installer is not supported on Windows"
            }
            "rpm installer" {
                throw "rpm installer is not supported on Windows"
            }
            "pkg installer" {
                throw "pkg installer is not supported on Windows"
            }
            "msi installer" {
                $Filename  = $Filenames[$Choice]
                $SavedFile = Join-Path $TmpDir $Filename
                if (-not (Get-RemoteFile $Urls[$Choice] $SavedFile)) {
                    throw "failed downloading msi installer"
                }
                Write-Debug "downloaded: $SavedFile"
                Write-Host "Launching MSI installer..."
                Start-Process msiexec.exe -ArgumentList @("/i", $SavedFile) -Wait
            }
            "exe installer" {
                $Filename  = $Filenames[$Choice]
                $SavedFile = Join-Path $TmpDir $Filename
                if (-not (Get-RemoteFile $Urls[$Choice] $SavedFile)) {
                    throw "failed downloading exe installer"
                }
                Write-Debug "downloaded: $SavedFile"
                Write-Host "Launching EXE installer..."
                Start-Process -FilePath $SavedFile -Wait
            }
            "tar.gz" {
                $Filename    = $Filenames[$Choice]
                $ArchivePath = Join-Path $TmpDir $Filename
                if (-not (Get-RemoteFile $Urls[$Choice] $ArchivePath)) {
                    throw "failed downloading tar.gz archive"
                }
                Write-Debug "downloaded: $ArchivePath"

                if (Get-Command tar -ErrorAction SilentlyContinue) {
                    tar -xzf $ArchivePath
                } elseif (Get-Command 7z -ErrorAction SilentlyContinue) {
                    7z x $ArchivePath
                    $TarFile = $ArchivePath -replace '\.gz$', ''
                    if (Test-Path $TarFile) { 7z x $TarFile; Remove-Item $TarFile -Force }
                } else {
                    throw "No extraction tool found. Please install tar or 7-Zip."
                }

                if (-not (Install-ArchiveContents -InstallPrefix $InstallPrefix -IsInteractive $IsInteractive)) { return }
            }
            "zip" {
                $Filename    = $Filenames[$Choice]
                $ArchivePath = Join-Path $TmpDir $Filename
                if (-not (Get-RemoteFile $Urls[$Choice] $ArchivePath)) {
                    throw "failed downloading zip archive"
                }
                Write-Debug "downloaded: $ArchivePath"
                Expand-Archive -Path $ArchivePath -DestinationPath $TmpDir -Force
                if (-not (Install-ArchiveContents -InstallPrefix $InstallPrefix -IsInteractive $IsInteractive)) { return }
            }
            default {
                throw "invalid filetype: $FileType"
            }
        }
    } finally {
        Pop-Location
        if (Test-Path $TmpDir) {
            Write-Host "Removing $TmpDir"
            Remove-Item $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

Invoke-PipeInstall
{% else %}
#------------------------------------------------------------------------------
# No Assets Available
#------------------------------------------------------------------------------
throw "no assets found"
{% endif %}
