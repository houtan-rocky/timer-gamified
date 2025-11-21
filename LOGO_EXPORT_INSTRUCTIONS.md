# Logo Export Instructions

## Created SVG Files

I've created two SVG logo files:
1. **logo-box-art.svg** - 1:1 Box art (1080x1080px)
2. **logo-poster-art.svg** - 2:3 Poster art (720x1080px)

## How to Convert SVG to PNG

### Option 1: Using Online Tools (Easiest)

1. **CloudConvert** (https://cloudconvert.com/svg-to-png)
   - Upload the SVG file
   - Set output size to:
     - Box art: 1080x1080 or 2160x2160
     - Poster art: 720x1080 or 1440x2160
   - Download PNG

2. **Convertio** (https://convertio.co/svg-png/)
   - Upload SVG
   - Set dimensions
   - Convert and download

### Option 2: Using Inkscape (Free Desktop Tool)

1. Download Inkscape: https://inkscape.org
2. Open the SVG file
3. Go to File → Export PNG Image
4. Set the exact dimensions:
   - Box art: 1080x1080 or 2160x2160
   - Poster art: 720x1080 or 1440x2160
5. Click Export

### Option 3: Using Command Line (if you have ImageMagick)

```bash
# Box art - 1080x1080
magick logo-box-art.svg -resize 1080x1080 logo-box-art-1080.png

# Box art - 2160x2160 (high resolution)
magick logo-box-art.svg -resize 2160x2160 logo-box-art-2160.png

# Poster art - 720x1080
magick logo-poster-art.svg -resize 720x1080 logo-poster-art-720.png

# Poster art - 1440x2160 (high resolution)
magick logo-poster-art.svg -resize 1440x2160 logo-poster-art-1440.png
```

### Option 4: Using Node.js (if you have it installed)

```bash
npm install -g svg2png-cli
svg2png logo-box-art.svg --output logo-box-art-1080.png --width 1080 --height 1080
svg2png logo-poster-art.svg --output logo-poster-art-720.png --width 720 --height 1080
```

## Required Sizes for Microsoft Store

### Box Art (Required - 1:1)
- **1080 x 1080px** (standard)
- **2160 x 2160px** (high resolution, optional but recommended)

### Poster Art (Recommended - 2:3)
- **720 x 1080px** (standard)
- **1440 x 2160px** (high resolution, optional but recommended)

## Design Details

The logos feature:
- **Dark background** (#0f0f0f) matching your app theme
- **Blue accent color** (#396cd8) matching your app's primary color
- **Orange highlights** (#ffa500) for timer indicators
- **Timer circle** with progress indicator
- **Gamification elements** (stars and checkmark badge)
- **App name** "TIMER GAMIFIED" (can be removed if you prefer)

## Customization

You can edit the SVG files to:
- Change colors
- Remove or modify text
- Adjust star positions
- Change the timer progress percentage
- Modify the overall design

Open the SVG files in any text editor or vector graphics tool (Inkscape, Figma, Adobe Illustrator) to customize.

## File Size Check

After conversion, ensure PNG files are:
- Less than 50 MB (they should be much smaller)
- Exact dimensions as specified
- PNG format (not JPEG)

## Next Steps

1. Convert SVG files to PNG at required sizes
2. Verify file sizes are under 50 MB
3. Upload to Microsoft Store during submission
4. Test how they look in the Store preview

