"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const electron_1 = require("electron");
electron_1.contextBridge.exposeInMainWorld('electron', {
    openDashboard: () => electron_1.ipcRenderer.invoke('open-dashboard'),
    startServer: () => electron_1.ipcRenderer.invoke('start-server'),
    stopServer: () => electron_1.ipcRenderer.invoke('stop-server'),
    getServerStatus: () => electron_1.ipcRenderer.invoke('get-server-status'),
    onServerStatus: (callback) => {
        const subscription = (_event, status) => callback(status);
        electron_1.ipcRenderer.on('server-status', subscription);
        return () => electron_1.ipcRenderer.removeListener('server-status', subscription);
    }
});
