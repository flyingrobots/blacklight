import { app, BrowserWindow, Tray, nativeImage, Menu, ipcMain, shell } from 'electron';
import path from 'path';
import isDevFromModule from 'electron-is-dev';
import { spawn, exec, ChildProcess } from 'child_process';

const isDev = isDevFromModule && process.env.NODE_ENV !== 'production';

let tray: Tray | null = null;
let window: BrowserWindow | null = null;
let serverProcess: ChildProcess | null = null;

function createWindow() {
  window = new BrowserWindow({
    width: 320,
    height: 500,
    show: false,
    frame: false,
    resizable: false,
    alwaysOnTop: true,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      nodeIntegration: false,
      contextIsolation: true,
    },
  });

  const url = isDev ? 'http://localhost:5173' : `file://${path.join(__dirname, '../dist/index.html')}`;
  window.loadURL(url);

  window.on('blur', () => {
    if (window && !window.webContents.isDevToolsOpened()) {
      window.hide();
    }
  });
}

function createTray() {
  const iconPath = path.join(__dirname, '../assets/tray-icon.png');
  const icon = nativeImage.createFromPath(iconPath);
  
  // Set as template for macOS (automatically flips black/white for light/dark mode)
  icon.setTemplateImage(true);
  
  tray = new Tray(icon);
  tray.setToolTip('Blacklight Companion');
  
  // Add a title so it's visible even if the icon fails
  if (process.platform === 'darwin') {
    tray.setTitle('BL');
  }

  tray.on('click', () => toggleWindow());

  const contextMenu = Menu.buildFromTemplate([
    { label: 'Open Dashboard', click: () => shell.openExternal('http://localhost:3141') },
    { type: 'separator' },
    { label: 'Quit', click: () => {
      stopServer();
      app.quit();
    }},
  ]);
  
  tray.on('right-click', () => {
    tray?.popUpContextMenu(contextMenu);
  });
}

function toggleWindow() {
  if (!window) return;
  window.isVisible() ? window.hide() : showWindow();
}

function showWindow() {
  if (!window || !tray) return;
  const trayBounds = tray.getBounds();
  const windowBounds = window.getBounds();
  const x = Math.round(trayBounds.x + (trayBounds.width / 2) - (windowBounds.width / 2));
  const y = Math.round(trayBounds.y + trayBounds.height + 4);
  window.setPosition(x, y, false);
  window.show();
  window.focus();
}

async function startServer() {
  if (serverProcess) return;
  
  console.log('Cleaning up existing processes...');
  // Force kill any existing blacklight process
  exec('pkill -9 blacklight', () => {
    console.log('Starting Blacklight backend...');
    const rootDir = path.join(__dirname, '../..');
    
    serverProcess = spawn('cargo', ['run', '--bin', 'blacklight', '--', 'serve', '--no-open'], {
      cwd: rootDir,
      stdio: 'inherit',
      env: { ...process.env, RUST_LOG: 'info' }
    });

    serverProcess.on('exit', (code) => {
      console.log(`Backend exited with code ${code}`);
      serverProcess = null;
      window?.webContents.send('server-status', 'stopped');
    });

    window?.webContents.send('server-status', 'running');
  });
}

function stopServer() {
  if (serverProcess) {
    console.log('Stopping backend...');
    serverProcess.kill('SIGTERM');
    serverProcess = null;
    window?.webContents.send('server-status', 'stopped');
  }
}

app.whenReady().then(() => {
  createWindow();
  createTray();
  if (process.platform === 'darwin' && app.dock) {
    app.dock.hide();
  }
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    stopServer();
    app.quit();
  }
});

app.on('before-quit', () => stopServer());

ipcMain.handle('open-dashboard', () => shell.openExternal('http://localhost:3141'));
ipcMain.handle('start-server', () => startServer());
ipcMain.handle('stop-server', () => stopServer());
ipcMain.handle('get-server-status', () => serverProcess ? 'running' : 'stopped');
