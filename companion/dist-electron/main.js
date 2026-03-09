"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const electron_1 = require("electron");
const path_1 = __importDefault(require("path"));
const electron_is_dev_1 = __importDefault(require("electron-is-dev"));
const child_process_1 = require("child_process");
const isDev = electron_is_dev_1.default && process.env.NODE_ENV !== 'production';
let tray = null;
let window = null;
let serverProcess = null;
function createWindow() {
    window = new electron_1.BrowserWindow({
        width: 320,
        height: 500,
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
    const url = isDev ? 'http://localhost:5173' : `file://${path_1.default.join(__dirname, '../dist/index.html')}`;
    window.loadURL(url);
    window.on('blur', () => {
        if (window && !window.webContents.isDevToolsOpened()) {
            window.hide();
        }
    });
}
function createTray() {
    const iconPath = path_1.default.join(__dirname, '../assets/tray-icon.png');
    const icon = electron_1.nativeImage.createFromPath(iconPath);
    // Set as template for macOS (automatically flips black/white for light/dark mode)
    icon.setTemplateImage(true);
    tray = new electron_1.Tray(icon);
    tray.setToolTip('Blacklight Companion');
    // Add a title so it's visible even if the icon fails
    if (process.platform === 'darwin') {
        tray.setTitle('BL');
    }
    tray.on('click', () => toggleWindow());
    const contextMenu = electron_1.Menu.buildFromTemplate([
        { label: 'Open Dashboard', click: () => electron_1.shell.openExternal('http://localhost:3141') },
        { type: 'separator' },
        { label: 'Quit', click: () => {
                stopServer();
                electron_1.app.quit();
            } },
    ]);
    tray.on('right-click', () => {
        tray?.popUpContextMenu(contextMenu);
    });
}
function toggleWindow() {
    if (!window)
        return;
    window.isVisible() ? window.hide() : showWindow();
}
function showWindow() {
    if (!window || !tray)
        return;
    const trayBounds = tray.getBounds();
    const windowBounds = window.getBounds();
    const x = Math.round(trayBounds.x + (trayBounds.width / 2) - (windowBounds.width / 2));
    const y = Math.round(trayBounds.y + trayBounds.height + 4);
    window.setPosition(x, y, false);
    window.show();
    window.focus();
}
async function startServer() {
    if (serverProcess)
        return;
    console.log('Cleaning up existing processes...');
    // Force kill any existing blacklight process
    (0, child_process_1.exec)('pkill -9 blacklight', () => {
        console.log('Starting Blacklight backend...');
        const rootDir = path_1.default.join(__dirname, '../..');
        serverProcess = (0, child_process_1.spawn)('cargo', ['run', '--bin', 'blacklight', '--', 'serve', '--no-open'], {
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
electron_1.app.whenReady().then(() => {
    createWindow();
    createTray();
    if (process.platform === 'darwin' && electron_1.app.dock) {
        electron_1.app.dock.hide();
    }
});
electron_1.app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
        stopServer();
        electron_1.app.quit();
    }
});
electron_1.app.on('before-quit', () => stopServer());
electron_1.ipcMain.handle('open-dashboard', () => electron_1.shell.openExternal('http://localhost:3141'));
electron_1.ipcMain.handle('start-server', () => startServer());
electron_1.ipcMain.handle('stop-server', () => stopServer());
electron_1.ipcMain.handle('get-server-status', () => serverProcess ? 'running' : 'stopped');
