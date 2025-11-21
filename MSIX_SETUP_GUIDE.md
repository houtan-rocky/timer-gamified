# MSIX Setup Guide for Microsoft Store

## Overview

Your app needs to be packaged as MSIX and digitally signed to comply with Microsoft Store Policy 10.2.9. This guide will help you configure Tauri to build MSIX packages with proper signing.

## Step 1: Update Configuration

The `tauri.conf.json` has been updated with MSIX configuration. You need to update the following fields:

### Update Publisher Information

In `src-tauri/tauri.conf.json`, update:

1. **Publisher CN (Certificate Name):**
   ```json
   "publisher": "CN=YourPublisherName"
   ```
   Replace `YourPublisherName` with your actual publisher name (e.g., "CN=John Doe" or "CN=YourCompanyName")

2. **Publisher Display Name:**
   ```json
   "publisherDisplayName": "Your Publisher Display Name"
   ```
   Replace with your actual publisher display name (this appears in Add/Remove Programs)

### Example Configuration

```json
"publisher": "CN=John Doe",
"publisherDisplayName": "John Doe",
```

## Step 2: Code Signing Options

You have two options for code signing:

### Option A: Microsoft Trusted Signing (Recommended for Store)

Microsoft Trusted Signing is the easiest option for Microsoft Store apps:

1. **In Microsoft Partner Center:**
   - Go to your app submission
   - Navigate to "Packages" section
   - Microsoft will automatically sign your MSIX package when you upload it
   - No additional certificate needed!

2. **For local testing:**
   - You can build unsigned MSIX packages locally
   - Microsoft Store will sign them during submission

### Option B: Your Own Certificate

If you have your own code signing certificate:

1. **Get certificate thumbprint:**
   ```powershell
   Get-ChildItem -Path Cert:\CurrentUser\My | Where-Object {$_.Subject -like "*YourPublisherName*"} | Select-Object Thumbprint
   ```

2. **Update tauri.conf.json:**
   ```json
   "certificateThumbprint": "YOUR_CERTIFICATE_THUMBPRINT_HERE"
   ```

3. **For timestamping (optional but recommended):**
   ```json
   "timestampUrl": "http://timestamp.digicert.com"
   ```

## Step 3: Build MSIX Package

### Build Command

```bash
npm run tauri build
```

Or specifically for MSIX:

```bash
npm run tauri build -- --target msix
```

### Output Location

The MSIX package will be in:
```
src-tauri/target/release/bundle/msix/TimerGamified_0.1.0_x64.msix
```

## Step 4: Verify MSIX Package

### Check Package Info

```powershell
Get-AppxPackageManifest -Path "TimerGamified_0.1.0_x64.msix"
```

### Test Installation (Optional)

```powershell
Add-AppxPackage -Path "TimerGamified_0.1.0_x64.msix"
```

## Step 5: Submit to Microsoft Store

1. **Upload the MSIX package** in Partner Center
2. **Microsoft will automatically sign it** if you're using Trusted Signing
3. The package will be properly identified in Add/Remove Programs

## Important Notes

### Publisher Name Format

- Must match your Microsoft Partner Center account
- Format: `CN=YourName` or `CN=YourCompanyName`
- This appears in "Add or Remove Programs"

### App Identity

- The `identity.name` must be unique and match your Store app identity
- Current: `com.base.TimerGamified`
- This should match your app's identity in Partner Center

### Capabilities

The current configuration sets:
- `internetClient: false` - App doesn't need internet
- `privateNetworkClientServer: false` - App doesn't need local network

If your app needs these capabilities, set them to `true`.

## Troubleshooting

### "Publisher name not found"
- Make sure the publisher CN matches your Partner Center account
- For Trusted Signing, you can leave it as a placeholder - Microsoft will update it

### "Certificate not found"
- If using your own certificate, ensure the thumbprint is correct
- For Trusted Signing, leave `certificateThumbprint` as `null`

### "MSIX build fails"
- Make sure you have the Windows SDK installed
- Tauri 2.0 should handle MSIX automatically, but ensure you're using the latest version

## Next Steps

1. ✅ Update publisher information in `tauri.conf.json`
2. ✅ Build MSIX package: `npm run tauri build`
3. ✅ Upload to Microsoft Partner Center
4. ✅ Microsoft will sign it automatically (Trusted Signing)
5. ✅ Submit for certification

## References

- [Tauri MSIX Documentation](https://tauri.app/v2/guides/building/msix/)
- [Microsoft Trusted Signing](https://learn.microsoft.com/en-us/windows/msix/package/trusted-signing)
- [Microsoft Store Policy 10.2.9](https://learn.microsoft.com/en-us/windows/uwp/publish/store-policies)

