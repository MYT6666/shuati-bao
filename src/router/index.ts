import { createRouter, createWebHistory } from 'vue-router'

const routes = [
  { path: '/', name: 'home', component: () => import('../views/HomeView.vue') },
  { path: '/import/:bankId?', name: 'import', component: () => import('../views/ImportView.vue') },
  { path: '/practice/:bankId', name: 'practice', component: () => import('../views/PracticeView.vue') },
  { path: '/wrong/:bankId', name: 'wrong', component: () => import('../views/WrongView.vue') },
  { path: '/favorites/:bankId', name: 'favorites', component: () => import('../views/FavoritesView.vue') },
  { path: '/stats/:bankId', name: 'stats', component: () => import('../views/StatsView.vue') },
  { path: '/settings', name: 'settings', component: () => import('../views/SettingsView.vue') },
]

export default createRouter({ history: createWebHistory(), routes })
