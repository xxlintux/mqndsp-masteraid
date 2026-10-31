; ==============================================================================
; MQN DSP Masteraid - Inno Setup 6 Script
; Precision Analog-Modeled Mastering Console
; ==============================================================================

#define MyAppName "Masteraid"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "MQN DSP"
#define MyAppURL "https://mqndsp.com"
#define MyAppExeName "Masteraid"

[Setup]
AppId={{9F4C7031-3645-4E3B-93EE-C1470F6CCE61}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={commonpf64}\MQNdsp\Masteraid
DefaultGroupName=MQN DSP
DisableProgramGroupPage=yes
DisableWelcomePage=no
OutputDir=..\dist
OutputBaseFilename=Masteraid_Windows_Installer_v{#MyAppVersion}
SetupIconFile=assets\app_icon.ico
WizardImageFile=assets\wizard_side.bmp
WizardSmallImageFile=assets\wizard_small.bmp
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog commandline
VersionInfoVersion={#MyAppVersion}
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription=Masteraid Precision Analog Mastering Console Installer
VersionInfoCopyright=Copyright (C) 2026 MQN DSP

[Types]
Name: "full"; Description: "Complete Installation (CLAP, VST3, Presets, Manual)"
Name: "custom"; Description: "Custom Component Selection"; Flags: iscustom

[Components]
Name: "vst3"; Description: "VST3 Plugin (64-bit Audio Plugin)"; Types: full custom
Name: "clap"; Description: "CLAP Plugin (64-bit Modern Audio Plugin)"; Types: full custom
Name: "presets"; Description: "Factory & Default Mastering Presets"; Types: full custom
Name: "manual"; Description: "Masteraid Interactive User Reference Manual (PDF)"; Types: full custom

[Files]
; VST3 Plugin Bundle
Source: "..\target\bundled\Masteraid.vst3\*"; DestDir: "{code:GetVst3TargetDir}"; Flags: recursesubdirs createallsubdirs ignoreversion; Components: vst3

; CLAP Plugin
Source: "..\target\bundled\Masteraid.clap"; DestDir: "{code:GetClapTargetDir}"; DestName: "masteraid.clap"; Flags: ignoreversion; Components: clap

; Factory Presets
Source: "presets\*.mqnpreset"; DestDir: "{code:GetPresetsDir}"; Flags: ignoreversion uninsneveruninstall; Components: presets

; User Reference Manual PDF
Source: "..\assets\Masteraid_User_Manual.pdf"; DestDir: "{code:GetManualDir}"; Flags: ignoreversion; Components: manual
Source: "..\assets\Masteraid_User_Manual.pdf"; DestDir: "{app}"; Flags: ignoreversion; Components: manual

[Icons]
Name: "{group}\Masteraid Reference Manual"; Filename: "{app}\Masteraid_User_Manual.pdf"; Components: manual
Name: "{group}\Open Presets Folder"; Filename: "{code:GetPresetsDir}"; Components: presets
Name: "{group}\Uninstall Masteraid"; Filename: "{uninstallexe}"

[Code]
var
  PathsPage: TInputDirWizardPage;

function GetDefaultVst3Dir: String;
begin
  Result := ExpandConstant('{commoncf64}\VST3');
end;

function GetDefaultClapDir: String;
begin
  Result := ExpandConstant('{commoncf64}\CLAP');
end;

function GetDefaultPresetsDir: String;
begin
  Result := ExpandConstant('{userdocs}\MQNdsp\Masteraid\Presets');
end;

procedure InitializeWizard;
begin
  PathsPage := CreateInputDirPage(wpSelectComponents,
    'Select Target Installation Folders',
    'Where should Masteraid install plugin formats and presets?',
    'Review or customize the destination folders below (defaults are industry-standard 64-bit paths):',
    False, '');

  PathsPage.Add('VST3 Plugin Directory (creates Masteraid.vst3):');
  PathsPage.Add('CLAP Plugin Directory:');
  PathsPage.Add('Mastering Presets Directory:');

  PathsPage.Values[0] := GetDefaultVst3Dir;
  PathsPage.Values[1] := GetDefaultClapDir;
  PathsPage.Values[2] := GetDefaultPresetsDir;
end;

function ShouldSkipPage(PageID: Integer): Boolean;
begin
  Result := False;
end;

function GetVst3TargetDir(Param: String): String;
var
  Base: String;
begin
  Base := Trim(PathsPage.Values[0]);
  if Length(Base) = 0 then
    Base := GetDefaultVst3Dir;
  if Uppercase(ExtractFileName(RemoveBackslashUnlessRoot(Base))) = 'MASTERAID.VST3' then
    Result := Base
  else
    Result := AddBackslash(Base) + 'Masteraid.vst3';
end;

function GetClapTargetDir(Param: String): String;
var
  Base: String;
begin
  Base := Trim(PathsPage.Values[1]);
  if Length(Base) = 0 then
    Base := GetDefaultClapDir;
  if Uppercase(ExtractFileName(RemoveBackslashUnlessRoot(Base))) = 'MASTERAID.CLAP' then
    Result := ExtractFilePath(RemoveBackslashUnlessRoot(Base))
  else
    Result := Base;
end;

function GetPresetsDir(Param: String): String;
begin
  if Length(Trim(PathsPage.Values[2])) > 0 then
    Result := PathsPage.Values[2]
  else
    Result := GetDefaultPresetsDir;
end;

function GetManualDir(Param: String): String;
begin
  Result := ExtractFilePath(RemoveBackslashUnlessRoot(GetPresetsDir('')));
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Vst3Path: String;
begin
  if CurStep = ssInstall then
  begin
    Vst3Path := GetVst3TargetDir('');
    // Remove legacy single-file VST3 if it exists so the modern directory bundle can be created cleanly
    if FileExists(Vst3Path) then
    begin
      DeleteFile(Vst3Path);
    end;
  end;
end;
