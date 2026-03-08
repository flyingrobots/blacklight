// Theme is applied via CSS tokens — no runtime switching needed.
// This composable exists as a hook point if multi-theme support is added later.

export function useTheme() {
  return {
    name: 'dark',
  }
}
