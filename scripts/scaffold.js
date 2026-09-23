import fs from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const args = process.argv.slice(2);
if (args.length === 0) {
  console.error("Please provide a module name. Usage: npm run scaffold <module-name>");
  process.exit(1);
}

const moduleName = args[0].toLowerCase();
const srcDir = path.resolve(process.cwd(), 'src');
const moduleDir = path.resolve(srcDir, 'modules', moduleName);

const folders = [
  'components',
  'controllers',
  'design',
  'helpers',
  'middleware',
  'pages',
  'routes',
  'services',
  'types',
  'validators'
];

async function scaffold() {
  try {
    await fs.mkdir(moduleDir, { recursive: true });
    
    for (const folder of folders) {
      const folderPath = path.resolve(moduleDir, folder);
      await fs.mkdir(folderPath, { recursive: true });
      // Create a .gitkeep file so empty directories are tracked by git
      await fs.writeFile(path.resolve(folderPath, '.gitkeep'), '');
    }
    
    console.log(`Successfully scaffolded module '${moduleName}' at src/modules/${moduleName}`);
  } catch (err) {
    console.error("Error scaffolding module:", err);
    process.exit(1);
  }
}

scaffold();
