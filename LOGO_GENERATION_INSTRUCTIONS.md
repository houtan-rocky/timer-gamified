# Logo Generation Instructions

## Quick Start (Recommended)

### Option 1: Using Node.js Script (Easiest)

1. **Install the required dependency:**
   ```bash
   npm install sharp
   ```

2. **Run the generation script:**
   ```bash
   npm run generate-logos
   ```
   or
   ```bash
   node generate-logos.js
   ```

3. **Done!** All PNG logos will be generated in `src-tauri/icons/`:
   - `box-art-1080.png` (1080x1080) - **Required for Microsoft Store**
   - `box-art-2160.png` (2160x2160) - High resolution
   - `poster-art-720.png` (720x1080) - **Recommended for Microsoft Store**
   - `poster-art-1440.png` (1440x2160) - High resolution
   - `StoreLogo.png` (300x300) - Updated with your logo
   - All app icon sizes (32x32, 128x128, etc.)

### Option 2: Using Online Tools (No Installation)

If you don't want to install Node.js dependencies:

1. **Go to CloudConvert:** https://cloudconvert.com/svg-to-png

2. **For Box Art (1080x1080):**
   - Upload `logo-box-art.svg`
   - Set width: 1080, height: 1080
   - Convert and download as `box-art-1080.png`
   - Save to `src-tauri/icons/`

3. **For Box Art (2160x2160):**
   - Upload `logo-box-art.svg`
   - Set width: 2160, height: 2160
   - Convert and download as `box-art-2160.png`
   - Save to `src-tauri/icons/`

4. **For Poster Art (720x1080):**
   - Upload `logo-poster-art.svg`
   - Set width: 720, height: 1080
   - Convert and download as `poster-art-720.png`
   - Save to `src-tauri/icons/`

5. **For Poster Art (1440x2160):**
   - Upload `logo-poster-art.svg`
   - Set width: 1440, height: 2160
   - Convert and download as `poster-art-1440.png`
   - Save to `src-tauri/icons/`

6. **For StoreLogo.png (300x300):**
   - Upload `logo-box-art.svg`
   - Set width: 300, height: 300
   - Convert and download as `StoreLogo.png`
   - Save to `src-tauri/icons/` (overwrite existing)

### Option 3: Using Inkscape (Free Desktop Tool)

1. **Download Inkscape:** https://inkscape.org

2. **Open the SVG file** (`logo-box-art.svg` or `logo-poster-art.svg`)

3. **Export as PNG:**
   - Go to File → Export PNG Image
   - Set the exact dimensions:
     - Box art: 1080x1080 or 2160x2160
     - Poster art: 720x1080 or 1440x2160
   - Click Export

4. **Save to** `src-tauri/icons/` with the appropriate filename

## Required Files for Microsoft Store

### Box Art (Required - 1:1 ratio)
- **1080 x 1080px** - Standard resolution (required)
- **2160 x 2160px** - High resolution (optional but recommended)

### Poster Art (Recommended - 2:3 ratio)
- **720 x 1080px** - Standard resolution (recommended)
- **1440 x 2160px** - High resolution (optional but recommended)

## File Naming

After generation, your `src-tauri/icons/` folder should contain:
- `box-art-1080.png` - For Microsoft Store submission
- `box-art-2160.png` - High resolution version
- `poster-art-720.png` - For Microsoft Store submission
- `poster-art-1440.png` - High resolution version
- `StoreLogo.png` - Updated with your custom logo (replaces Tauri default)

## Verification

Before submitting to Microsoft Store:

1. ✅ Box art is exactly 1080x1080 or 2160x2160 pixels
2. ✅ Poster art is exactly 720x1080 or 1440x2160 pixels
3. ✅ All files are PNG format
4. ✅ File sizes are under 50 MB (they should be much smaller)
5. ✅ Logos are clear and readable
6. ✅ StoreLogo.png has been updated (not the default Tauri logo)

## Troubleshooting

### "sharp module not found"
Run: `npm install sharp`

### "Cannot find module"
Make sure you're in the project root directory when running the script.

### SVG files not found
Make sure `logo-box-art.svg` and `logo-poster-art.svg` are in the project root.

### Generated files are too large
PNG files should be under 50 MB. If they're larger, the SVG might be too complex. Consider simplifying the design.

## Next Steps

1. Generate all required logo sizes
2. Verify file sizes and dimensions
3. Upload to Microsoft Store during submission:
   - Box art: Upload `box-art-1080.png` or `box-art-2160.png`
   - Poster art: Upload `poster-art-720.png` or `poster-art-1440.png`
4. The app will automatically use the updated `StoreLogo.png` and other icons

