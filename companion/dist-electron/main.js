"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const electron_1 = require("electron");
const path_1 = __importDefault(require("path"));
const electron_is_dev_1 = __importDefault(require("electron-is-dev"));
let tray = null;
let window = null;
function createWindow() {
    window = new electron_1.BrowserWindow({
        width: 320,
        height: 450,
        show: false,
        frame: false,
        resizable: false,
        alwaysOnTop: true,
        webPreferences: {
            preload: path_1.default.join(__dirname, 'preload.js'),
            nodeIntegration: false,
            contextIsolation: true,
        },
    });
    if (electron_is_dev_1.default) {
        window.loadURL('http://localhost:5173');
        // window.webContents.openDevTools({ mode: 'detach' });
    }
    else {
        window.loadFile(path_1.default.join(__dirname, '../dist/index.html'));
    }
    window.on('blur', () => {
        if (window && !window.webContents.isDevToolsOpened()) {
            window.hide();
        }
    });
}
function createTray() {
    // Use a simple template icon for macOS
    const icon = electron_1.nativeImage.createFromPath(path_1.default.join(__dirname, '../assets/tray-icon.png'));
    tray = new electron_1.Tray(icon.isEmpty() ? electron_1.nativeImage.createEmpty() : icon);
    tray.setToolTip('Blacklight Companion');
    tray.on('click', () => {
        toggleWindow();
    });
    tray.on('right-click', () => {
        const contextMenu = electron_1.Menu.buildFromTemplate([
            { label: 'Open Dashboard', click: () => { } },
            { type: 'separator' },
            { label: 'Quit', click: () => electron_1.app.quit() },
        ]);
        tray?.popUpContextMenu(contextMenu);
    });
}
function toggleWindow() {
    if (!window)
        return;
    if (window.isVisible()) {
        window.hide();
    }
    else {
        showWindow();
    }
}
function showWindow() {
    if (!window || !tray)
        return;
    const trayBounds = tray.getBounds();
    const windowBounds = window.getBounds();
    // Center window horizontally under the tray icon
    const x = Math.round(trayBounds.x + (trayBounds.width / 2) - (windowBounds.width / 2));
    const y = Math.round(trayBounds.y + trayBounds.height + 4);
    window.setPosition(x, y, false);
    window.show();
    window.focus();
}
electron_1.app.whenReady().then(() => {
    createWindow();
    createTray();
    // Hide dock icon on macOS
    if (process.platform === 'darwin' && electron_1.app.dock) {
        electron_1.app.dock.hide();
    }
});
electron_1.app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
        electron_1.app.quit();
    }
});
// IPC handlers for controlling Blacklight
electron_1.ipcMain.handle('open-dashboard', () => {
    require('electron').shell.openExternal('http://localhost:3141');
});
