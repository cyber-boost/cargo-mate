use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone)]
pub struct InstallerGenTool;
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstallerConfig {
    name: String,
    version: String,
    description: String,
    author: String,
    platforms: Vec<String>,
    files: Vec<String>,
    dependencies: Vec<String>,
    scripts: HashMap<String, String>,
    metadata: HashMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeneratedInstaller {
    platform: String,
    files_created: Vec<String>,
    installer_size: u64,
    installer_path: String,
    metadata: HashMap<String, String>,
}
impl InstallerGenTool {
    pub fn new() -> Self {
        Self
    }
    fn parse_cargo_toml(&self, manifest_path: &str) -> Result<InstallerConfig> {
        let content = fs::read_to_string(manifest_path)
            .map_err(|e| ToolError::IoError(e))?;
        let cargo_toml: toml::Value = toml::from_str(&content)
            .map_err(|e| ToolError::TomlError(e))?;
        let package = cargo_toml
            .get("package")
            .and_then(|p| p.as_table())
            .ok_or_else(|| ToolError::InvalidArguments(
                "No [package] section in Cargo.toml".to_string(),
            ))?;
        let name = package
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();
        let version = package
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.1.0")
            .to_string();
        let description = package
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or(&format!("{} application", name))
            .to_string();
        let authors = package
            .get("authors")
            .and_then(|a| a.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.as_str())
            .unwrap_or("Unknown Author")
            .to_string();
        Ok(InstallerConfig {
            name,
            version,
            description,
            author: authors,
            platforms: vec![
                "windows".to_string(), "macos".to_string(), "linux".to_string()
            ],
            files: vec!["target/release/".to_string()],
            dependencies: vec![],
            scripts: HashMap::new(),
            metadata: HashMap::new(),
        })
    }
    fn generate_windows_installer(
        &self,
        config: &InstallerConfig,
        output_dir: &str,
    ) -> Result<GeneratedInstaller> {
        let installer_name = format!(
            "{}-{}-windows-setup.exe", config.name, config.version
        );
        let installer_path = Path::new(output_dir).join(&installer_name);
        let nsis_script = self.generate_nsis_script(config)?;
        let nsis_path = Path::new(output_dir).join("installer.nsi");
        fs::create_dir_all(output_dir)?;
        fs::write(&nsis_path, nsis_script)?;
        let inno_script = self.generate_inno_setup_script(config)?;
        let inno_path = Path::new(output_dir).join("installer.iss");
        fs::write(&inno_path, inno_script)?;
        let ps_script = self.generate_powershell_installer(config)?;
        let ps_path = Path::new(output_dir).join("install.ps1");
        fs::write(&ps_path, ps_script)?;
        let files_created = vec![
            nsis_path.to_string_lossy().to_string(), inno_path.to_string_lossy()
            .to_string(), ps_path.to_string_lossy().to_string(),
        ];
        let mut metadata = HashMap::new();
        metadata
            .insert(
                "installer_type".to_string(),
                "Windows (NSIS/Inno Setup/PowerShell)".to_string(),
            );
        metadata.insert("compression".to_string(), "LZMA".to_string());
        metadata.insert("uac_support".to_string(), "Yes".to_string());
        Ok(GeneratedInstaller {
            platform: "windows".to_string(),
            files_created,
            installer_size: 0,
            installer_path: installer_path.to_string_lossy().to_string(),
            metadata,
        })
    }
    fn generate_macos_installer(
        &self,
        config: &InstallerConfig,
        output_dir: &str,
    ) -> Result<GeneratedInstaller> {
        let installer_name = format!("{}-{}-macos.pkg", config.name, config.version);
        let installer_path = Path::new(output_dir).join(&installer_name);
        let dist_xml = self.generate_distribution_xml(config)?;
        let dist_path = Path::new(output_dir).join("Distribution");
        fs::create_dir_all(output_dir)?;
        fs::write(&dist_path, dist_xml)?;
        let postinstall_script = self.generate_postinstall_script(config)?;
        let postinstall_path = Path::new(output_dir).join("postinstall");
        fs::write(&postinstall_path, postinstall_script)?;
        let dmg_script = self.generate_dmg_script(config)?;
        let dmg_path = Path::new(output_dir).join("create_dmg.sh");
        fs::write(&dmg_path, dmg_script)?;
        let files_created = vec![
            dist_path.to_string_lossy().to_string(), postinstall_path.to_string_lossy()
            .to_string(), dmg_path.to_string_lossy().to_string(),
        ];
        let mut metadata = HashMap::new();
        metadata.insert("installer_type".to_string(), "macOS (.pkg/.dmg)".to_string());
        metadata
            .insert("code_signing".to_string(), "Required for distribution".to_string());
        metadata.insert("gatekeeper".to_string(), "Compatible".to_string());
        Ok(GeneratedInstaller {
            platform: "macos".to_string(),
            files_created,
            installer_size: 0,
            installer_path: installer_path.to_string_lossy().to_string(),
            metadata,
        })
    }
    fn generate_linux_installer(
        &self,
        config: &InstallerConfig,
        output_dir: &str,
    ) -> Result<GeneratedInstaller> {
        let installer_name = format!("{}-{}-linux.run", config.name, config.version);
        let installer_path = Path::new(output_dir).join(&installer_name);
        let deb_dir = Path::new(output_dir).join("deb");
        fs::create_dir_all(&deb_dir)?;
        let control_file = self.generate_deb_control(config)?;
        fs::write(deb_dir.join("control"), control_file)?;
        let postinst_script = self.generate_deb_postinst(config)?;
        fs::write(deb_dir.join("postinst"), postinst_script)?;
        let rpm_spec = self.generate_rpm_spec(config)?;
        let rpm_path = Path::new(output_dir).join("package.spec");
        fs::write(&rpm_path, rpm_spec)?;
        let appimage_script = self.generate_appimage_script(config)?;
        let appimage_path = Path::new(output_dir).join("create_appimage.sh");
        fs::write(&appimage_path, appimage_script)?;
        let files_created = vec![
            deb_dir.join("control").to_string_lossy().to_string(), deb_dir
            .join("postinst").to_string_lossy().to_string(), rpm_path.to_string_lossy()
            .to_string(), appimage_path.to_string_lossy().to_string(),
        ];
        let mut metadata = HashMap::new();
        metadata
            .insert(
                "installer_type".to_string(),
                "Linux (DEB/RPM/AppImage)".to_string(),
            );
        metadata.insert("package_managers".to_string(), "apt, yum, dnf".to_string());
        metadata.insert("self_extracting".to_string(), "Yes".to_string());
        Ok(GeneratedInstaller {
            platform: "linux".to_string(),
            files_created: files_created.into_iter().map(|p| p.to_string()).collect(),
            installer_size: 0,
            installer_path: installer_path.to_string_lossy().to_string(),
            metadata,
        })
    }
    fn generate_nsis_script(&self, config: &InstallerConfig) -> Result<String> {
        Ok(
            format!(
                r#"
;NSIS Installer Script for {name}
;Generated by CargoMate InstallerGen

!include "MUI2.nsh"
!include "FileFunc.nsh"

Name "{name} {version}"
OutFile "{name}-{version}-setup.exe"
Unicode True
InstallDir "$PROGRAMFILES\{name}"
InstallDirRegKey HKCU "Software\{name}" ""
RequestExecutionLevel admin

!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

Section "MainSection" SEC01
    SetOutPath "$INSTDIR"
    SetOverwrite ifnewer

    ; Add your application files here
    ; File /r "path\to\your\app\*.*"

    WriteRegStr HKCU "Software\{name}" "" $INSTDIR
    WriteUninstaller "$INSTDIR\Uninstall.exe"

    ; Create desktop shortcut
    CreateShortCut "$DESKTOP\{name}.lnk" "$INSTDIR\{name}.exe"

    ; Add to PATH
    EnVar::SetHKCU
    EnVar::AddValue "PATH" "$INSTDIR"
SectionEnd

Section -AdditionalIcons
    WriteIniStr "$INSTDIR\website.url" "InternetShortcut" "URL" "{website}"
    CreateShortCut "$SMPROGRAMS\{name}\Website.lnk" "$INSTDIR\website.url"
    CreateShortCut "$SMPROGRAMS\{name}\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
SectionEnd

Section -Post
    WriteUninstaller "$INSTDIR\uninstall.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\{name}" "DisplayName" "{name}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\{name}" "UninstallString" "$INSTDIR\uninstall.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\{name}" "DisplayVersion" "{version}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\{name}" "Publisher" "{author}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\{name}" "DisplayIcon" "$INSTDIR\{name}.exe"
SectionEnd

Section Uninstall
    Delete "$INSTDIR\website.url"
    Delete "$INSTDIR\uninstall.exe"
    Delete "$DESKTOP\{name}.lnk"
    RMDir /r "$SMPROGRAMS\{name}"

    ; Remove from PATH
    EnVar::SetHKCU
    EnVar::DeleteValue "PATH" "$INSTDIR"

    RMDir /r "$INSTDIR"
    DeleteRegKey /ifempty HKCU "Software\{name}"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\{name}"
SectionEnd
"#,
                name = config.name, version = config.version, author = config.author,
                website = "https://example.com"
            ),
        )
    }
    fn generate_inno_setup_script(&self, config: &InstallerConfig) -> Result<String> {
        Ok(
            format!(
                r#"
;Inno Setup Script for {name}
;Generated by CargoMate InstallerGen

#define MyAppName "{name}"
#define MyAppVersion "{version}"
#define MyAppPublisher "{author}"
#define MyAppURL "https:
#define MyAppExeName "{name}.exe"

[Setup]
AppId={{{{MyAppName}}}}
AppName={{#MyAppName}}
AppVersion={{#MyAppVersion}}
AppPublisher={{#MyAppPublisher}}
AppPublisherURL={{#MyAppURL}}
AppSupportURL={{#MyAppURL}}
AppUpdatesURL={{#MyAppURL}}
DefaultDirName={{pf}}\{{#MyAppName}}
DefaultGroupName={{#MyAppName}}
AllowNoIcons=yes
OutputDir=userdocs
OutputBaseFilename={name}-{version}-setup
SetupIconFile=
Compression=lzma
SolidCompression=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{{cm:CreateDesktopIcon}}"; GroupDescription: "{{cm:AdditionalIcons}}"; Flags: unchecked
Name: "quicklaunchicon"; Description: "{{cm:CreateQuickLaunchIcon}}"; GroupDescription: "{{cm:AdditionalIcons}}"; Flags: unchecked

[Files]
Source: "path\to\your\app\*"; DestDir: "{{app}}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{{group}}\\{{#MyAppName}}"; Filename: "{{app}}\\{{#MyAppExeName}}"
Name: "{{group}}\\{{cm:UninstallProgram,{{#MyAppName}}}}"; Filename: "{{uninstallexe}}"
Name: "{{commondesktop}}\\{{#MyAppName}}"; Filename: "{{app}}\\{{#MyAppExeName}}"; Tasks: desktopicon
Name: "{{userappdata}}\\Microsoft\\Internet Explorer\\Quick Launch\\{{#MyAppName}}"; Filename: "{{app}}\\{{#MyAppExeName}}"; Tasks: quicklaunchicon

[Run]
Filename: "{{app}}\\{{#MyAppExeName}}"; Description: "{{cm:LaunchProgram,{{#MyAppName}}}}"; Flags: nowait postinstall skipifsilent

[Registry]
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "PATH"; ValueData: "{{olddata}};{{app}}"; Check: NeedsAddPath(ExpandConstant('{{app}}'))

[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'PATH', OrigPath) then begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
  if Result = True then
    Result := Pos(Param + ';', OrigPath + ';') = 0;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then begin
    if NeedsAddPath(ExpandConstant('{{app}}')) then begin
      RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'PATH', ExpandConstant('{{olddata}};{{app}}'));
    end;
  end;
end;
"#,
                name = config.name, version = config.version, author = config.author
            ),
        )
    }
    fn generate_powershell_installer(&self, config: &InstallerConfig) -> Result<String> {
        Ok(
            format!(
                r#"# PowerShell Installer for {name}
# Generated by CargoMate InstallerGen

param(
    [string]$InstallPath = "$env:ProgramFiles\{name}",
    [switch]$Force
)

$ErrorActionPreference = "Stop"

Write-Host "Installing {name} {version}..." -ForegroundColor Green

# Check if already installed
if (Test-Path "$InstallPath\{name}.exe" -and -not $Force) {{
    Write-Host "{name} is already installed. Use -Force to reinstall." -ForegroundColor Yellow
    exit 0
}}

# Create installation directory
if (-not (Test-Path $InstallPath)) {{
    New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null
}}

Write-Host "Extracting files to $InstallPath..." -ForegroundColor Cyan

# Copy application files (replace with actual file copy logic)
# Copy-Item -Path "path\to\your\app\*" -Destination $InstallPath -Recurse -Force

Write-Host "Creating shortcuts..." -ForegroundColor Cyan

# Create desktop shortcut
$desktopPath = [Environment]::GetFolderPath("Desktop")
$shortcutPath = Join-Path $desktopPath "{name}.lnk"
$wshShell = New-Object -ComObject WScript.Shell
$shortcut = $wshShell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = "$InstallPath\{name}.exe"
$shortcut.WorkingDirectory = $InstallPath
$shortcut.Save()

# Add to PATH
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$InstallPath*") {{
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$InstallPath", "User")
    Write-Host "Added $InstallPath to PATH" -ForegroundColor Green
}}

# Create uninstaller
$uninstallerScript = @"
param([switch]$Force)

if (-not `$Force) {{
    `$response = Read-Host "Are you sure you want to uninstall {name}? (y/N)"
    if (`$response -ne "y" -and `$response -ne "Y") {{
        exit 0
    }}
}}

Write-Host "Uninstalling {name}..." -ForegroundColor Yellow

# Remove desktop shortcut
Remove-Item "$desktopPath\{name}.lnk" -ErrorAction SilentlyContinue

# Remove from PATH
`$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
`$newPath = ($currentPath -split ';' | Where-Object {{ `$_ -ne "$InstallPath" }}) -join ';'
[Environment]::SetEnvironmentVariable("PATH", `$newPath, "User")

# Remove installation directory
Remove-Item $InstallPath -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "Uninstallation complete." -ForegroundColor Green
"@

$uninstallerPath = "$InstallPath\uninstall.ps1"
Set-Content -Path $uninstallerPath -Value $uninstallerScript

Write-Host "Installation complete!" -ForegroundColor Green
Write-Host ""
Write-Host "To uninstall, run:" -ForegroundColor Cyan
Write-Host "powershell -ExecutionPolicy Bypass -File $uninstallerPath" -ForegroundColor White
"#,
                name = config.name, version = config.version
            ),
        )
    }
    fn generate_distribution_xml(&self, config: &InstallerConfig) -> Result<String> {
        Ok(
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<installer-gui-script minSpecVersion="1">
    <title>{name} {version}</title>
    <organization>{author}</organization>
    <options customize="never" require-scripts="false"/>
    <welcome file="welcome.html" mime-type="text/html"/>
    <license file="license.html" mime-type="text/html"/>
    <script>
        function isRootUser() {{
            return system.gid === 0;
        }}
    </script>
    <pkg-ref id="{name}.pkg"/>
    <choices-outline>
        <line choice="default">
            <line choice="{name}"/>
        </line>
    </choices-outline>
    <choice id="default"/>
    <choice id="{name}" title="{name}" description="{description}">
        <pkg-ref id="{name}.pkg"/>
    </choice>
    <pkg-ref id="{name}.pkg" version="{version}" auth="Root">file:./{name}.pkg</pkg-ref>
</installer-gui-script>
"#,
                name = config.name, version = config.version, author = config.author,
                description = config.description
            ),
        )
    }
    fn generate_postinstall_script(&self, config: &InstallerConfig) -> Result<String> {
        Ok(
            format!(
                r#"#!/bin/bash

# Post-installation script for {name}
# Generated by CargoMate InstallerGen

set -e

echo "Running post-installation tasks for {name}..."

# Create symbolic links if needed
# ln -sf "/usr/local/bin/{name}" "/usr/bin/{name}"

# Set permissions
# chmod +x "/usr/local/bin/{name}"

# Update desktop database
# update-desktop-database 2>/dev/null || true

echo "Post-installation complete!"
"#,
                name = config.name
            ),
        )
    }
    fn generate_dmg_script(&self, config: &InstallerConfig) -> Result<String> {
        Ok(
            format!(
                r#"#!/bin/bash

# DMG Creation Script for {name}
# Generated by CargoMate InstallerGen

set -e

APP_NAME="{name}"
APP_VERSION="{version}"
DMG_NAME="${{APP_NAME}}-${{APP_VERSION}}.dmg"

echo "Creating DMG for ${{APP_NAME}}..."

# Create temporary directory structure
mkdir -p dmg_temp
cp -r "path/to/your/app" dmg_temp/

# Create Applications symlink
ln -s /Applications dmg_temp/

# Create DMG
hdiutil create \
    -volname "${{APP_NAME}} ${{APP_VERSION}}" \
    -srcfolder dmg_temp \
    -ov -format UDZO \
    "${{DMG_NAME}}"

# Clean up
rm -rf dmg_temp

echo "DMG created: ${{DMG_NAME}}"
"#,
                name = config.name, version = config.version
            ),
        )
    }
    fn generate_deb_control(&self, config: &InstallerConfig) -> Result<String> {
        Ok(
            format!(
                r#"Package: {name}
Version: {version}
Section: utils
Priority: optional
Architecture: amd64
Depends: libc6 (>= 2.15)
Maintainer: {author}
Description: {description}
Homepage: https:
"#,
                name = config.name, version = config.version, author = config.author,
                description = config.description
            ),
        )
    }
    fn generate_deb_postinst(&self, config: &InstallerConfig) -> Result<String> {
        Ok(
            format!(
                r#"#!/bin/bash

# DEB postinst script for {name}
# Generated by CargoMate InstallerGen

set -e

case "$1" in
    configure)
        # Add user to appropriate groups if needed
        # usermod -a -G dialout $SUDO_USER || true

        # Update system databases
        ldconfig 2>/dev/null || true

        # Update desktop database
        update-desktop-database 2>/dev/null || true

        echo "{name} {version} has been installed successfully!"
        echo "You may need to restart your session for changes to take effect."
        ;;
    abort-upgrade|abort-remove|abort-deconfigure)
        ;;
    *)
        echo "postinst called with unknown argument \`$1'" >&2
        exit 1
        ;;
esac

exit 0
"#,
                name = config.name, version = config.version
            ),
        )
    }
    fn generate_rpm_spec(&self, config: &InstallerConfig) -> Result<String> {
        Ok(
            format!(
                r#"Name:           {name}
Version:        {version}
Release:        1
Summary:        {description}
License:        MIT
URL:            https:
Source0:        %{{name}}-%{{version}}.tar.gz
BuildArch:      x86_64

Requires:       glibc >= 2.15

%description
{description}

%prep
%autosetup

%build
# Build commands here
# cargo build --release

%install
# Install commands here
# mkdir -p %{{buildroot}}%{{_bindir}}
# cp target/release/{name} %{{buildroot}}%{{_bindir}}/

%files
%{{_bindir}}/{name}

%changelog
* {date} {author} - {version}-1
- Initial package
"#,
                name = config.name, version = config.version, description = config
                .description, author = config.author, date = chrono::Utc::now()
                .format("%a %b %d %Y")
            ),
        )
    }
    fn generate_appimage_script(&self, config: &InstallerConfig) -> Result<String> {
        Ok(
            format!(
                r#"#!/bin/bash

# AppImage Creation Script for {name}
# Generated by CargoMate InstallerGen

set -e

APP_NAME="{name}"
APP_VERSION="{version}"
APPIMAGE_NAME="${{APP_NAME}}-${{APP_VERSION}}.AppImage"

echo "Creating AppImage for ${{APP_NAME}}..."

# Create AppDir structure
mkdir -p AppDir/usr/bin
mkdir -p AppDir/usr/share/applications
mkdir -p AppDir/usr/share/icons/hicolor/256x256/apps

# Copy application binary
cp "path/to/your/app" AppDir/usr/bin/

# Create desktop file
cat > AppDir/usr/share/applications/${{APP_NAME}}.desktop << EOF
[Desktop Entry]
Name={name}
Exec={name}
Icon={name}
Type=Application
Categories=Utility;
EOF

# Create icon (placeholder)
# cp "path/to/icon.png" AppDir/usr/share/icons/hicolor/256x256/apps/${{APP_NAME}}.png

# Create AppRun script
cat > AppDir/AppRun << EOF
#!/bin/bash
HERE="$(dirname "$(readlink -f "${{0}}")")"
export PATH="${{HERE}}/usr/bin":$PATH
exec "${{HERE}}/usr/bin/{name}" "$@"
EOF

chmod +x AppDir/AppRun

# Download and run appimagetool
if ! command -v appimagetool &> /dev/null; then
    echo "Installing appimagetool..."
    wget -O appimagetool "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
    chmod +x appimagetool
fi

./appimagetool AppDir "${{APPIMAGE_NAME}}"

# Clean up
rm -rf AppDir

echo "AppImage created: ${{APPIMAGE_NAME}}"
"#,
                name = config.name, version = config.version
            ),
        )
    }
    fn display_report(
        &self,
        installers: &[GeneratedInstaller],
        output_format: OutputFormat,
        verbose: bool,
    ) {
        match output_format {
            OutputFormat::Human => {
                println!(
                    "\n📦 {} - Platform-Specific Installers Generated",
                    "CargoMate InstallerGen".bold().blue()
                );
                println!("{}", "═".repeat(70).blue());
                for installer in installers {
                    println!(
                        "\n🏗️  {} Installer:", installer.platform.to_uppercase()
                        .green()
                    );
                    println!("  📁 Output: {}", installer.installer_path);
                    if verbose {
                        println!("  📄 Files Created:");
                        for file in &installer.files_created {
                            println!("    • {}", file);
                        }
                        println!("  📋 Metadata:");
                        for (key, value) in &installer.metadata {
                            println!("    • {}: {}", key, value);
                        }
                    }
                }
                println!("\n💡 Next Steps:");
                println!("  1. Review generated installer files");
                println!("  2. Customize with your application files");
                println!("  3. Test installation on target platforms");
                println!("  4. Sign installers for distribution");
                println!("\n🔧 Build Commands:");
                for installer in installers {
                    match installer.platform.as_str() {
                        "windows" => {
                            println!("  • NSIS: makensis.exe installer.nsi");
                            println!("  • Inno Setup: iscc.exe installer.iss");
                        }
                        "macos" => {
                            println!("  • DMG: bash create_dmg.sh");
                            println!(
                                "  • PKG: productbuild --distribution Distribution --package-path ."
                            );
                        }
                        "linux" => {
                            println!("  • DEB: dpkg-deb --build deb package.deb");
                            println!("  • RPM: rpmbuild -ba package.spec");
                            println!("  • AppImage: bash create_appimage.sh");
                        }
                        _ => {}
                    }
                }
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(installers)
                    .unwrap_or_else(|_| "[]".to_string());
                println!("{}", json);
            }
            OutputFormat::Table => {
                println!(
                    "{:<12} {:<15} {:<20} {:<10}", "Platform", "Type", "Output File",
                    "Files"
                );
                println!("{}", "─".repeat(70));
                for installer in installers {
                    println!(
                        "{:<12} {:<15} {:<20} {:<10}", installer.platform, installer
                        .metadata.get("installer_type").unwrap_or(& "Unknown"
                        .to_string()), std::path::Path::new(& installer.installer_path)
                        .file_name().unwrap_or_default().to_string_lossy(), installer
                        .files_created.len().to_string()
                    );
                }
            }
        }
    }
}
impl Tool for InstallerGenTool {
    fn name(&self) -> &'static str {
        "installer-gen"
    }
    fn description(&self) -> &'static str {
        "Generate platform-specific installers"
    }
    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about(
                "Generate platform-specific installers for your Rust application. \
                        Supports Windows (NSIS/Inno Setup/PowerShell), macOS (.pkg/.dmg), \
                        and Linux (DEB/RPM/AppImage) installers.

EXAMPLES:
    cm tool installer-gen --platforms windows,macos
    cm tool installer-gen --config installer.toml --output installers/
    cm tool installer-gen --sign --notarize",
            )
            .args(
                &[
                    Arg::new("platforms")
                        .long("platforms")
                        .short('p')
                        .help("Target platforms (windows,macos,linux)")
                        .default_value("windows,macos,linux"),
                    Arg::new("config")
                        .long("config")
                        .short('c')
                        .help("Configuration file (TOML)")
                        .default_value("Cargo.toml"),
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("Output directory for installers")
                        .default_value("installers/"),
                    Arg::new("name")
                        .long("name")
                        .short('n')
                        .help("Application name (from Cargo.toml)"),
                    Arg::new("version")
                        .long("version")
                        .short('v')
                        .help("Application version (from Cargo.toml)"),
                    Arg::new("sign")
                        .long("sign")
                        .help("Sign installers (requires certificates)")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("notarize")
                        .long("notarize")
                        .help("Notarize macOS installers")
                        .action(clap::ArgAction::SetTrue),
                ],
            )
            .args(&common_options())
    }
    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let platforms: Vec<String> = matches
            .get_one::<String>("platforms")
            .unwrap()
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        let config_file = matches.get_one::<String>("config").unwrap();
        let output_dir = matches.get_one::<String>("output").unwrap();
        let sign = matches.get_flag("sign");
        let notarize = matches.get_flag("notarize");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");
        println!(
            "📦 {} - Generating Platform-Specific Installers", "CargoMate InstallerGen"
            .bold().blue()
        );
        let mut config = if Path::new(config_file).exists() {
            self.parse_cargo_toml(config_file)?
        } else {
            return Err(
                ToolError::InvalidArguments(
                    format!("Config file {} not found", config_file),
                ),
            );
        };
        if let Some(name) = matches.get_one::<String>("name") {
            config.name = name.clone();
        }
        if let Some(version) = matches.get_one::<String>("version") {
            config.version = version.clone();
        }
        fs::create_dir_all(output_dir)?;
        let mut generated_installers = Vec::new();
        for platform in &platforms {
            let platform_output = Path::new(output_dir).join(platform);
            fs::create_dir_all(&platform_output)?;
            match platform.as_str() {
                "windows" => {
                    match self
                        .generate_windows_installer(
                            &config,
                            &platform_output.to_string_lossy(),
                        )
                    {
                        Ok(installer) => {
                            generated_installers.push(installer);
                            if verbose {
                                println!("✅ Generated Windows installer files");
                            }
                        }
                        Err(e) => {
                            println!("❌ Failed to generate Windows installer: {}", e);
                        }
                    }
                }
                "macos" => {
                    match self
                        .generate_macos_installer(
                            &config,
                            &platform_output.to_string_lossy(),
                        )
                    {
                        Ok(installer) => {
                            generated_installers.push(installer);
                            if verbose {
                                println!("✅ Generated macOS installer files");
                            }
                        }
                        Err(e) => {
                            println!("❌ Failed to generate macOS installer: {}", e);
                        }
                    }
                }
                "linux" => {
                    match self
                        .generate_linux_installer(
                            &config,
                            &platform_output.to_string_lossy(),
                        )
                    {
                        Ok(installer) => {
                            generated_installers.push(installer);
                            if verbose {
                                println!("✅ Generated Linux installer files");
                            }
                        }
                        Err(e) => {
                            println!("❌ Failed to generate Linux installer: {}", e);
                        }
                    }
                }
                _ => {
                    println!("⚠️  Unsupported platform: {}", platform);
                }
            }
        }
        if generated_installers.is_empty() {
            return Err(
                ToolError::ExecutionFailed(
                    "No installers were successfully generated".to_string(),
                ),
            );
        }
        if sign {
            println!("\n🔐 Signing installers...");
            println!(
                "⚠️  Installer signing requires platform-specific certificates"
            );
        }
        if notarize {
            println!("\n📝 Notarizing macOS installers...");
            println!("⚠️  Notarization requires Apple Developer account");
        }
        self.display_report(&generated_installers, output_format, verbose);
        Ok(())
    }
}
impl Default for InstallerGenTool {
    fn default() -> Self {
        Self::new()
    }
}