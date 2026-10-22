import type { MouseEvent, ReactNode } from "react";
import type { DragControls } from "framer-motion";
import type { ClipboardEntry, Locale } from "../../shared/types";

export interface QuickPasteHint {
  slot: number;
  combo: string;
}

export interface ClipboardItemProps {
  item: ClipboardEntry;
  /** Current position in the combined pinned + history list. */
  navigationIndex: number;
  isSelected: boolean;
  /** Shown in the wide layout's detail pane, whose action bar targets it. */
  isDetailFocused?: boolean;
  windowPinned: boolean;
  isSensitiveHidden: boolean;
  isRevealed: boolean;
  /** Offers show / hide: a sensitive entry while 隐藏隐私 is on. */
  canReveal: boolean;
  isEditingTags: boolean;
  isEditingNote: boolean;
  noteDraft?: string;
  onNoteDraftChange?: (note: string) => void;
  tagInput: string;
  /** Tags used elsewhere in history; shown as quick-pick when editing tags */
  tagSuggestions?: string[];
  theme: string;
  language: Locale;
  t: (key: string) => string;
  isAIProcessing?: boolean;
  /** A follow-up question from the AI waiting for the user's answer. */
  aiQuestion?: string;
  /** An AI result waiting for the user to replace the entry or discard it. */
  aiPendingResult?: string;
  aiEnabled?: boolean;
  tagColors?: Record<string, string>;
  aiOptionsOpen?: boolean;
  richTextSnapshotPreview?: boolean;
  showSourceAppIcon?: boolean;
  sensitiveMaskPrefixVisible?: number;
  sensitiveMaskSuffixVisible?: number;
  sensitiveMaskEmailDomain?: boolean;
  quickPasteHint?: QuickPasteHint;

  onSelect: () => void;
  onCopy: (withFormat?: boolean) => void;
  onToggleReveal: (e: MouseEvent) => void;
  onOpen: (e: MouseEvent) => void;
  onTogglePin: (e: MouseEvent) => void;
  onDelete: (e: MouseEvent) => void;
  onToggleTagEditor: (e: MouseEvent) => void;
  onToggleNoteEditor: (e: MouseEvent) => void;
  onNoteSave: (note: string) => void;
  onNoteEditCancel?: () => void;
  onTagInput: (val: string) => void;
  onTagAdd: () => void;
  /** Pick an existing tag from the suggestion list (typically closes editor after add) */
  onTagPick?: (tag: string) => void;
  /** Close tag editor without adding (e.g. Escape) */
  onTagEditCancel?: () => void;
  onTagDelete: (tag: string) => void;
  onAIAction?: (type: string) => void;
  onAIOptionsToggle?: () => void;
  onInputSubmit?: (val: string) => void;
  onAIApply?: () => void;
  onAIDiscard?: () => void;
  dragControls?: DragControls;
  id?: string;
  disableLayout?: boolean;
}

export type ClipboardRenderItem = (
  item: ClipboardEntry,
  index: number,
  isFirst: boolean
) => ReactNode;

export interface VirtualClipboardListProps {
  items: ClipboardEntry[];
  renderItem: ClipboardRenderItem;
  onLoadMore?: () => void;
  hasMore: boolean;
  isLoading: boolean;
  selectedIndex: number;
  isKeyboardMode: boolean;
  onScroll?: (offset: number) => void;
  compactMode: boolean;
  header?: ReactNode;
  loadingLabel: string;
  loadMoreLabel: string;
}

export interface VirtualClipboardListHandle {
  scrollToItem: (index: number) => void;
  scrollToTop: () => void;
  resetAfterIndex: (index: number) => void;
}
