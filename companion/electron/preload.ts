import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('electron', {
  openDashboard: () => ipcRenderer.invoke('open-dashboard'),
});
