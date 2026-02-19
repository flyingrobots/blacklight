import { ref, onMounted, onUnmounted, isRef, type Ref } from 'vue'
import { gsap } from 'gsap'
import { ScrollToPlugin } from 'gsap/ScrollToPlugin'

gsap.registerPlugin(ScrollToPlugin)

export function useKeyboardNavigation(itemCount: Ref<number> | number, onSelect: (index: number) => void) {
  const selectedIndex = ref(-1)

  function getItemCount(): number {
    return isRef(itemCount) ? itemCount.value : itemCount
  }

  function handleKeyDown(e: KeyboardEvent) {
    const target = e.target as HTMLElement
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') {
      return
    }

    const count = getItemCount()

    if (e.key === 'ArrowDown' || e.key === 'j') {
      e.preventDefault()
      selectedIndex.value = Math.min(selectedIndex.value + 1, count - 1)
      scrollToSelected()
    } else if (e.key === 'ArrowUp' || e.key === 'k') {
      e.preventDefault()
      selectedIndex.value = Math.max(selectedIndex.value - 1, 0)
      scrollToSelected()
    } else if (e.key === 'Enter') {
      if (selectedIndex.value >= 0) {
        onSelect(selectedIndex.value)
      }
    } else if (e.key === '/') {
      e.preventDefault()
      const searchInput = document.querySelector('.search-input') as HTMLInputElement
      searchInput?.focus()
    }
  }

  function scrollToSelected() {
    const el = document.querySelector(`[data-nav-index="${selectedIndex.value}"]`) as HTMLElement
    if (el) {
      const rect = el.getBoundingClientRect()
      const scrollY = window.scrollY || window.pageYOffset
      
      // Calculate the absolute top of the element
      const elTop = rect.top + scrollY
      
      // Calculate the target scroll position to center the element
      // Target = elTop - (viewportHeight / 2) + (elHeight / 2)
      const targetScroll = elTop - (window.innerHeight / 2) + (rect.height / 2)

      gsap.to(window, {
        scrollTo: { y: targetScroll, autoKill: false },
        duration: 0.4,
        ease: 'power2.out'
      })
    }
  }

  onMounted(() => {
    window.addEventListener('keydown', handleKeyDown)
  })

  onUnmounted(() => {
    window.removeEventListener('keydown', handleKeyDown)
  })

  return { selectedIndex }
}
