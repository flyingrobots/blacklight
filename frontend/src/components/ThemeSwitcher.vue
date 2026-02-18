<template>
  <div class="theme-switcher">
    <button
      v-for="(theme, i) in themes"
      :key="theme.name"
      :class="['theme-dot', { active: currentThemeIndex === i }]"
      :title="theme.name"
      @click="setTheme(i)"
    >
      <span
        class="dot-inner"
        :style="{ background: `linear-gradient(135deg, ${theme.colors[2]}, ${theme.colors[4]})` }"
      ></span>
    </button>
  </div>
</template>

<script setup lang="ts">
import { useTheme } from '@/composables/useTheme'

const { themes, currentThemeIndex, setTheme } = useTheme()
</script>

<style scoped>
.theme-switcher {
  display: flex;
  gap: 0.375rem;
  align-items: center;
}
.theme-dot {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  border: 2px solid transparent;
  background: none;
  padding: 0;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: border-color 0.2s, transform 0.15s;
}
.theme-dot:hover {
  transform: scale(1.2);
}
.theme-dot.active {
  border-color: var(--bl-text);
}
.dot-inner {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  display: block;
}
</style>
