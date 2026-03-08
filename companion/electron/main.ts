import { app, BrowserWindow, Tray, nativeImage, Menu, ipcMain, screen } from 'electron';
import path from 'path';
import isDev from 'electron-is-dev';

let tray: Tray | null = null;
let window: BrowserWindow | null = null;

function createWindow() {
  window = new BrowserWindow({
    width: 320,
    height: 450,
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

  if (isDev) {
    window.loadURL('http://localhost:5173');
    // window.webContents.openDevTools({ mode: 'detach' });
  } else {
    window.loadFile(path.join(__dirname, '../dist/index.html'));
  }

  window.on('blur', () => {
    if (window && !window.webContents.isDevToolsOpened()) {
      window.hide();
    }
  });
}

function createTray() {
  // Use a simple template icon for macOS
  const icon = nativeImage.createFromPath(path.join(__dirname, '../assets/tray-icon.png'));
  tray = new Tray(icon.isEmpty() ? nativeImage.createEmpty() : icon);
  tray.setToolTip('Blacklight Companion');

  tray.on('click', () => {
    toggleWindow();
  });

  tray.on('right-click', () => {
    const contextMenu = Menu.buildFromTemplate([
      { label: 'Open Dashboard', click: () => { /* open in browser */ } },
      { type: 'separator' },
      { label: 'Quit', click: () => app.quit() },
    ]);
    tray?.popUpContextMenu(contextMenu);
  });
}

function toggleWindow() {
  if (!window) return;
  if (window.isVisible()) {
    window.hide();
  } else {
    showWindow();
  }
}

function showWindow() {
  if (!window || !tray) return;
  
  const trayBounds = tray.getBounds();
  const windowBounds = window.getBounds();
  
  // Center window horizontally under the tray icon
  const x = Math.round(trayBounds.x + (trayBounds.width / 2) - (windowBounds.width / 2));
  const y = Math.round(trayBounds.y + trayBounds.height + 4);
  
  window.setPosition(x, y, false);
  window.show();
  window.focus();
}

app.whenReady().then(() => {
  createWindow();
  createTray();
  
  // Hide dock icon on macOS
  if (process.platform === 'darwin' && app.dock) {
    app.dock.hide();
  }
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

// IPC handlers for controlling Blacklight
ipcMain.handle('open-dashboard', () => {
  require('electron').shell.openExternal('http://localhost:3141');
});
