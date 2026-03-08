import { createRouter, createWebHistory } from 'vue-router'

import Home from '@/views/Home.vue'
import Sessions from '@/views/Sessions.vue'
import SessionDetail from '@/views/SessionDetail.vue'
import Projects from '@/views/Projects.vue'
import Search from '@/views/Search.vue'
import Insights from '@/views/Insights.vue'
import Operations from '@/views/Operations.vue'
import Digests from '@/views/Digests.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    { path: '/', component: Home },
    { path: '/sessions', component: Sessions },
    { path: '/sessions/:id', component: SessionDetail },
    { path: '/projects', component: Projects },
    { path: '/search', component: Search },
    { path: '/insights', component: Insights },
    { path: '/digests', component: Digests },
    { path: '/operations', component: Operations },
  ],
})

export default router
