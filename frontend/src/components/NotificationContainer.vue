<template>
  <div class="notification-container">
    <TransitionGroup @enter="onEnter" @leave="onLeave">
      <NotificationToast
        v-for="toast in visible"
        :key="toast.id"
        :toast="toast"
        @dismiss="dismiss(toast.id)"
      />
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
import { gsap } from 'gsap'
import NotificationToast from './NotificationToast.vue'
import { useNotifications } from '@/composables/useNotifications'

const { visible, dismiss } = useNotifications()

function onEnter(el: Element, done: () => void) {
  gsap.fromTo(
    el,
    { x: 80, opacity: 0 },
    { x: 0, opacity: 1, duration: 0.3, ease: 'power2.out', onComplete: done },
  )
}

function onLeave(el: Element, done: () => void) {
  gsap.to(el, {
    x: 40,
    opacity: 0,
    duration: 0.25,
    ease: 'power2.in',
    onComplete: done,
  })
}
</script>

<style scoped>
.notification-container {
  position: fixed;
  top: 1rem;
  right: 1.5rem;
  z-index: 9500;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
</style>
