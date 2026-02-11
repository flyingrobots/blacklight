<template>
  <div class="app">
    <nav class="sidebar">
      <div class="logo">
        <img :src="logoUrl" alt="Blacklight" class="logo-img" />
      </div>
      <ul class="nav-links">
        <li><router-link to="/">Dashboard</router-link></li>
        <li><router-link to="/sessions">Sessions</router-link></li>
        <li><router-link to="/projects">Projects</router-link></li>
        <li><router-link to="/search">Search</router-link></li>
        <li><router-link to="/analytics">Analytics</router-link></li>
        <li><router-link to="/files">Files</router-link></li>
        <li><router-link to="/storage">Storage</router-link></li>
      </ul>
    </nav>
    <main class="content">
      <router-view />
    </main>
  </div>
</template>

<script setup lang="ts">
import logoUrl from '@/assets/BLACKLIGHT.svg'
</script>

<style>
.app {
  display: flex;
  min-height: 100vh;
}
.sidebar {
  width: 220px;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
  padding: 1rem 0;
  flex-shrink: 0;
  position: fixed;
  height: 100vh;
  overflow-y: auto;
}
.logo {
  padding: 0.75rem 1.25rem 1.25rem;
  border-bottom: 1px solid var(--border);
  margin-bottom: 0.5rem;
}
.logo-img {
  width: 100%;
  filter: invert(1) brightness(0.85) sepia(1) hue-rotate(180deg) saturate(3);
}
.nav-links {
  list-style: none;
}
.nav-links li a {
  display: block;
  padding: 0.5rem 1.25rem;
  color: var(--text-secondary);
  transition: all 0.15s;
}
.nav-links li a:hover,
.nav-links li a.router-link-active {
  color: var(--text);
  background: var(--bg-tertiary);
  text-decoration: none;
}
.content {
  flex: 1;
  margin-left: 220px;
  padding: 2rem;
  max-width: 1200px;
}
</style>
