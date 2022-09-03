UniSBOM is a tool to build a [software bill of materials](https://www.cisa.gov/sbom) on any platform with a unified data format.

**Work in progress**

## Support

### MacOS

Uses `system_profiler` to build a list of installed applications and kernel extensions. 

<details>
<summary><b>Example JSON</b></summary>

```json
[{
		"kind": "OS",
		"name": "macOS",
		"id": "macOS",
		"version": "macOS 12.5.1 (21G83)",
		"path": "/",
		"modified": "1970-01-01T00:00:00Z",
		"publishers": [
			"Apple Code Signing Certification Authority",
			"Apple Root CA"
		]
  	},
	{
		"kind": "Application",
		"name": "UnmountAssistantAgent",
		"id": "UnmountAssistantAgent",
		"version": "5.0",
		"path": "/System/Library/CoreServices/UnmountAssistantAgent.app",
		"modified": "2022-08-11T06:44:38Z",
		"publishers": [
			"Software Signing",
			"Apple Code Signing Certification Authority",
			"Apple Root CA"
		]
	},
	{
		"kind": "Application",
		"name": "Google Drive",
		"id": "Google Drive",
		"version": "62.0",
		"path": "/Applications/Google Drive.app",
		"modified": "2022-08-25T12:50:40Z",
		"publishers": [
			"Developer ID Application: Google LLC (EQHXZ8M8AV)",
			"Developer ID Certification Authority",
			"Apple Root CA"
		]
	},
	{
		"kind": "Driver",
		"name": "VBoxDrv",
		"id": "org.virtualbox.kext.VBoxDrv",
		"version": "6.1.36",
		"path": "/Library/Application Support/VirtualBox/VBoxDrv.kext",
		"modified": "2022-07-19T22:00:36Z",
		"publishers": [
			"Developer ID Application: Oracle America, Inc. (VB5E2TV963), Developer ID Certification Authority, Apple Root CA"
		]
	},
	{
		"kind": "Driver",
		"name": "IOSCSIBlockCommandsDevice",
		"id": "com.apple.iokit.IOSCSIBlockCommandsDevice",
		"version": "456.140.3",
		"path": "/System/Library/Extensions/IOSCSIArchitectureModelFamily.kext/Contents/PlugIns/IOSCSIBlockCommandsDevice.kext",
		"modified": "2022-08-11T06:44:38Z",
		"publishers": [
			"Software Signing, Apple Code Signing Certification Authority, Apple Root CA"
		]
	}
]
```
</details>

### Windows

Uses registry keys to build a list of installed applications and `driverquery` for the drivers.

<details>
<summary><b>Example JSON</b></summary>

```json
[{
		"kind": "Application",
		"name": "Google Chrome",
		"id": "Google Chrome",
		"version": "105.0.5195.54",
		"path": "C:\\Program Files\\Google\\Chrome\\Application",
		"modified": "2022-09-02T13:45:10Z",
		"publishers": [
			"Google LLC"
		]
	},
	{
		"kind": "Application",
		"name": "Microsoft Azure Storage Emulator - v5.10",
		"id": "Microsoft Azure Storage Emulator - v5.10",
		"version": "5.10.19227.2113",
		"path": "C:\\ProgramData\\Microsoft\\VisualStudio\\Packages\\Microsoft.Azure.Storage.Emulator.Msi,version=5.10.19227.2113\\",
		"modified": "2021-02-15T08:37:36Z",
		"publishers": [
			"Microsoft Corporation"
		]
	},
	{
		"kind": "Driver",
		"name": "Intel(R) Serial IO I2C Driver v2",
		"id": "iaLPSS2i_I2C_CNL",
		"version": "30.100.1929.1",
		"path": "C:\\Windows\\system32\\drivers\\iaLPSS2i_I2C_CNL.sys",
		"modified": "2019-07-14T22:12:12Z",
		"publishers": []
	},
	{
		"kind": "Driver",
		"name": "Intel(R) Serial IO I2C Driver v2",
		"id": "iaLPSS2i_I2C_GLK",
		"version": "30.100.1820.1",
		"path": "C:\\Windows\\system32\\drivers\\iaLPSS2i_I2C_GLK.sys",
		"modified": "2018-05-15T22:46:02Z",
		"publishers": []
	}
]
```
</details>


### GNU/Linux

TODO

## Building

```sh
cargo build --release
```

## Running 

Print the SBOM as a textual summary:

```sh
./target/release/unisbom
```

Output all details as JSON:

```sh
./target/release/unisbom --format json
```

## Other options

Run `unisbom --help` for the complete list of options. 

## License

This project is made with ♥  by [@evilsocket](https://twitter.com/evilsocket) and it is released under the GPL3 license.