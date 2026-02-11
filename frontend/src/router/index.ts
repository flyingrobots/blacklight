import { createRouter, createWebHistory } from 'vue-router'

import Dashboard from '@/views/Dashboard.vue'
import Sessions from '@/views/Sessions.vue'
import SessionDetail from '@/views/SessionDetail.vue'
import Projects from '@/views/Projects.vue'
import Search from '@/views/Search.vue'
import Analytics from '@/views/Analytics.vue'
import Storage from '@/views/Storage.vue'
import Files from '@/views/Files.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: Dashboard },
    { path: '/sessions', component: Sessions },
    { path: '/sessions/:id', component: SessionDetail },
    { path: '/projects', component: Projects },
    { path: '/search', component: Search },
    { path: '/analytics', component: Analytics },
    { path: '/storage', component: Storage },
    { path: '/files', component: Files },
  ],
})

export default router
