// Main-window view state shared by the two panes.
export const ui = $state({
  notes: false,
  tab: 'new' as 'new' | 'following' | 'settings',
  /** Nothing followed yet: the room shows its first-light scene. */
  empty: false,
  /** Settings should put the cursor in the address field. */
  paste: false,
});
