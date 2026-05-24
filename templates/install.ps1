#requires -version 3.0

#{# template engine Tera #}

#------------------------------------------------------------------------------
# 01) Runtime Setup
#------------------------------------------------------------------------------
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
{% if (assets | length > 0) %}
$RUN_DIRECTORY = $PWD.Path
$_QUIET = {{ quiet | escape_shell }}
$_FORCE = {{ force | escape_shell }}
$_CANONICAL_BINARY_NAME = {{ app | escape_shell }}

$_E_GENERIC_ERROR = 1

#------------------------------------------------------------------------------
# 02) Temporary Workspace and Exit Cleanup
#------------------------------------------------------------------------------
$_TMPDIR = New-TemporaryFile | ForEach-Object {
    Remove-Item $_
    New-Item -ItemType Directory -Path $_
}
Set-Location $_TMPDIR.FullName

$cleanup = {
    if (Test-Path $_TMPDIR) {
        [Console]::Error.WriteLine("Removing $_TMPDIR")
        Remove-Item $_TMPDIR -Recurse -Force -ErrorAction SilentlyContinue
    }
    Set-Location $RUN_DIRECTORY
}

Register-EngineEvent PowerShell.Exiting -Action $cleanup | Out-Null
trap { & $cleanup; break }

#------------------------------------------------------------------------------
# 03) Interactive Choice Prompt
#------------------------------------------------------------------------------
function Get-UserChoice {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Choices,
        [switch]$AllowNone,
        [switch]$AllowQuit
    )

    if ($Choices.Count -eq 0) {
        [Console]::Error.WriteLine("no choices provided")
        exit 1
    }

    $idx = 1
    foreach ($choice in $Choices) {
        Write-Host "`t$idx)`t$choice"
        $idx++
    }

    if ($AllowNone) {
        Write-Host "`tn)`tnone"
    }
    if ($AllowQuit) {
        Write-Host "`tq)`tquit"
    }

    do {
        $userInput = Read-Host "Enter choice"

        if ($AllowQuit -and ($userInput -eq "q" -or $userInput -eq "quit")) {
            return "q"
        }
        if ($AllowNone -and ($userInput -eq "n" -or $userInput -eq "none")) {
            return "n"
        }

        $choiceNum = 0
        if ([int]::TryParse($userInput, [ref]$choiceNum)) {
            if ($choiceNum -ge 1 -and $choiceNum -le $Choices.Count) {
                return ($choiceNum - 1)
            }
        }

        Write-Host "Invalid choice. Please try again." -ForegroundColor Red
    } while ($true)
}

#------------------------------------------------------------------------------
# 04) Download Helper
#------------------------------------------------------------------------------
function Get-WebContent {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Url,
        [string]$OutFile
    )

    try {
        if ($OutFile) {
            Invoke-WebRequest -Uri $Url -OutFile $OutFile -UseBasicParsing -ErrorAction Stop
            return $true
        } else {
            return (Invoke-WebRequest -Uri $Url -UseBasicParsing -ErrorAction Stop).Content
        }
    }
    catch {
        [Console]::Error.WriteLine("Failed to download from $Url`: $_")
        if ($OutFile) {
            return $false
        }
        return $null
    }
}

#------------------------------------------------------------------------------
# 05) Rendered Asset Arrays
#------------------------------------------------------------------------------
$_urls = @({% for asset in assets %}"{{ asset.url | escape_shell }}"{% if not loop.last %}, {% endif %}{% endfor %})
$_filenames = @({% for asset in assets %}"{{ asset.name | escape_shell }}"{% if not loop.last %}, {% endif %}{% endfor %})
$_filetypes = @({% for asset in assets %}"{{ asset.filetype | escape_shell }}"{% if not loop.last %}, {% endif %}{% endfor %})
$_printables = @({% for asset in assets %}"{{ asset.name ~ " (" ~ asset.filetype ~ ")" | escape_shell }}"{% if not loop.last %}, {% endif %}{% endfor %})

#------------------------------------------------------------------------------
# 06) Asset Selection
#------------------------------------------------------------------------------
Write-Host "Please select one of the following:"
$choice = Get-UserChoice -Choices $_printables -AllowQuit

#------------------------------------------------------------------------------
# 07) Selection Validation
#------------------------------------------------------------------------------
if ($choice -eq "q" -or $choice -eq "n") {
    exit 0
}

if ($choice -lt 0 -or $choice -ge $_urls.Count) {
    [Console]::Error.WriteLine("invalid choice: $choice")
    exit 100
}

#------------------------------------------------------------------------------
# 08) Download and Install Dispatch
#------------------------------------------------------------------------------
Write-Host "Downloading from $($_urls[$choice]) to $_TMPDIR"
$_type = $_filetypes[$choice]

switch ($_type) {
    "binary" {
        $filename = $_filenames[$choice]
        $saved_file = Join-Path $_TMPDIR.FullName $filename
        if (-not (Get-WebContent -Url $_urls[$choice] -OutFile $saved_file)) {
            [Console]::Error.WriteLine("failed downloading binary asset")
            exit 100
        }

        if ([string]::IsNullOrWhiteSpace($_CANONICAL_BINARY_NAME)) {
            $binary_name = Read-Host "enter alternate binary name (default: $filename)"
            if ([string]::IsNullOrWhiteSpace($binary_name)) {
                $binary_name = $filename
            }
        }
        else {
            $binary_name = $_CANONICAL_BINARY_NAME
        }

        $default_bin_dir = Join-Path $RUN_DIRECTORY "bin"
        $binary_dir = Read-Host "enter alternate binary directory (default: $default_bin_dir)"
        if ([string]::IsNullOrWhiteSpace($binary_dir)) {
            $binary_dir = $default_bin_dir
        }
        if (-not (Test-Path $binary_dir)) {
            New-Item -ItemType Directory -Path $binary_dir -Force | Out-Null
        }

        $dest_path = Join-Path $binary_dir $binary_name
        Copy-Item $saved_file $dest_path -Force
        Write-Host "Installed $binary_name to $dest_path"
    }
    "deb installer" {
        [Console]::Error.WriteLine("deb installer is not supported on Windows")
        exit 100
    }
    "rpm installer" {
        [Console]::Error.WriteLine("rpm installer is not supported on Windows")
        exit 100
    }
    "pkg installer" {
        [Console]::Error.WriteLine("pkg installer is not supported on Windows")
        exit 100
    }
    "msi installer" {
        $filename = $_filenames[$choice]
        $saved_file = Join-Path $_TMPDIR.FullName $filename
        if (-not (Get-WebContent -Url $_urls[$choice] -OutFile $saved_file)) {
            [Console]::Error.WriteLine("failed downloading msi installer")
            exit 100
        }
        Write-Host "Launching MSI installer..."
        Start-Process msiexec.exe -ArgumentList "/i `"$saved_file`"" -Wait
    }
    "exe installer" {
        $filename = $_filenames[$choice]
        $saved_file = Join-Path $_TMPDIR.FullName $filename
        if (-not (Get-WebContent -Url $_urls[$choice] -OutFile $saved_file)) {
            [Console]::Error.WriteLine("failed downloading exe installer")
            exit 100
        }
        Write-Host "Launching EXE installer..."
        Start-Process -FilePath $saved_file -Wait
    }
    "tar.gz" {
        $filename = $_filenames[$choice]

        # Download and extract tar.gz
        $archive_path = Join-Path $_TMPDIR.FullName $filename
        if (-not (Get-WebContent -Url $_urls[$choice] -OutFile $archive_path)) {
            [Console]::Error.WriteLine("failed downloading tar.gz archive")
            exit 100
        }

        # Extract using tar (available in Windows 10 1803+) or 7-Zip if available
        if (Get-Command tar -ErrorAction SilentlyContinue) {
            tar -xzf $archive_path
        }
        elseif (Get-Command 7z -ErrorAction SilentlyContinue) {
            7z x $archive_path
            $tar_file = $archive_path -replace '\.gz$', ''
            if (Test-Path $tar_file) {
                7z x $tar_file
                Remove-Item $tar_file -Force
            }
        }
        else {
            [Console]::Error.WriteLine("No extraction tool found. Please install tar or 7-Zip")
            exit 100
        }

        # Find executable files
        $executable_files = @()
        Get-ChildItem -Recurse -File | ForEach-Object {
            if ($_.Extension -eq ".exe" -or $_.Extension -eq "" -or $_.Name -notmatch '\.') {
                $executable_files += $_.FullName
            }
        }

        if ($executable_files.Count -eq 0) {
            [Console]::Error.WriteLine("no executable files found in archive")
            exit 100
        } else {
            $choices = Get-UserChoice -Choices $executable_files -AllowQuit

            if ($choices -ne "q") {
                $selected_file = $executable_files[$choices]
                $default_bin_dir = Join-Path $RUN_DIRECTORY "bin"
                if (-not (Test-Path $default_bin_dir)) {
                    New-Item -ItemType Directory -Path $default_bin_dir -Force | Out-Null
                }

                $dest_name = Split-Path $selected_file -Leaf
                $dest_path = Join-Path $default_bin_dir $dest_name
                Copy-Item $selected_file $dest_path -Force
                Write-Host "Installed $dest_name to $dest_path"
            }
        }
    }
    default {
        [Console]::Error.WriteLine("invalid filetype: $_type")
        exit 100
    }
}
{% else %}
#------------------------------------------------------------------------------
# 09) No Assets Available
#------------------------------------------------------------------------------
[Console]::Error.WriteLine("no assets found")
exit 100
{% endif %}

# cleanup execution for non-engine-exit completion paths
& $cleanup
