// Main-window view state shared by the two panes.
export const ui = $state({
  notes: false,
  tab: 'new' as 'new' | 'following' | 'settings',
  /** Nothing followed yet: the room shows its first-light scene. */
  empty: false,
  /** Settings should put the cursor in the address field. */
  paste: false,
  /** A show just added: Following opens it. */
  reveal: null as number | null,
  /** A show to open in Following (its name or art was chosen while listening). */
  showing: null as number | null,
  /** What an import just brought in: Following leads with it until you move on. */
  arrived: null as { note: string; ids: number[]; unreached: string[] } | null,
});
