// Script to generate PNG logos from SVG files
// Requires: npm install sharp
// Usage: node generate-logos.js

import sharp from 'sharp';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

async function generateLogos() {
  const svgBoxArt = path.join(__dirname, 'logo-box-art.svg');
  const svgPosterArt = path.join(__dirname, 'logo-poster-art.svg');
  const iconsDir = path.join(__dirname, 'src-tauri', 'icons');

  // Ensure icons directory exists
  if (!fs.existsSync(iconsDir)) {
    fs.mkdirSync(iconsDir, { recursive: true });
  }

  try {
    // Generate Box Art - 1080x1080 (required)
    console.log('Generating box-art-1080.png (1080x1080)...');
    await sharp(svgBoxArt)
      .resize(1080, 1080)
      .png()
      .toFile(path.join(iconsDir, 'box-art-1080.png'));

    // Generate Box Art - 2160x2160 (high resolution)
    console.log('Generating box-art-2160.png (2160x2160)...');
    await sharp(svgBoxArt)
      .resize(2160, 2160)
      .png()
      .toFile(path.join(iconsDir, 'box-art-2160.png'));

    // Generate Poster Art - 720x1080 (standard)
    console.log('Generating poster-art-720.png (720x1080)...');
    await sharp(svgPosterArt)
      .resize(720, 1080)
      .png()
      .toFile(path.join(iconsDir, 'poster-art-720.png'));

    // Generate Poster Art - 1440x2160 (high resolution)
    console.log('Generating poster-art-1440.png (1440x2160)...');
    await sharp(svgPosterArt)
      .resize(1440, 2160)
      .png()
      .toFile(path.join(iconsDir, 'poster-art-1440.png'));

    // Also generate StoreLogo.png (300x300) from box art
    console.log('Generating StoreLogo.png (300x300)...');
    await sharp(svgBoxArt)
      .resize(300, 300)
      .png()
      .toFile(path.join(iconsDir, 'StoreLogo.png'));

    // Generate other icon sizes for the app
    console.log('Generating app icons...');
    const iconSizes = [32, 44, 71, 89, 107, 128, 142, 150, 284, 310];
    
    for (const size of iconSizes) {
      const filename = size === 128 ? '128x128.png' : `Square${size}x${size}Logo.png`;
      await sharp(svgBoxArt)
        .resize(size, size)
        .png()
        .toFile(path.join(iconsDir, filename));
      console.log(`  Generated ${filename} (${size}x${size})`);
    }

    // Generate main icon.png (512x512 is common)
    console.log('Generating icon.png (512x512)...');
    await sharp(svgBoxArt)
      .resize(512, 512)
      .png()
      .toFile(path.join(iconsDir, 'icon.png'));

    console.log('\n✅ All logos generated successfully!');
    console.log('\nFiles created in src-tauri/icons/:');
    console.log('  - box-art-1080.png (1080x1080) - Required for Microsoft Store');
    console.log('  - box-art-2160.png (2160x2160) - High resolution');
    console.log('  - poster-art-720.png (720x1080) - Recommended for Microsoft Store');
    console.log('  - poster-art-1440.png (1440x2160) - High resolution');
    console.log('  - StoreLogo.png (300x300) - Updated');
    console.log('  - All app icon sizes updated');

  } catch (error) {
    console.error('Error generating logos:', error);
    console.error('\nMake sure you have installed sharp: npm install sharp');
    process.exit(1);
  }
}

generateLogos();

