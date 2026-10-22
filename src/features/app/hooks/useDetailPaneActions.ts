import { useMemo, type Dispatch, type MouseEvent, type SetStateAction } from "react";
import type { ClipboardEntry } from "../../../shared/types";
import type { DetailPaneActions } from "../../clipboard/components/ClipboardDetailPane";

interface UseDetailPaneActionsOptions {
  /** Entries in display order, to pick the neighbour after a delete. */
  items: ClipboardEntry[];
  focusDetailItem: (id: number) => void;
  copyToClipboard: (
    id: number,
    content: string,
    contentType: string,
    pasteWithFormat?: boolean,
    isPinned?: boolean,
    tags?: string[]
  ) => Promise<void>;
  openContent: (item: ClipboardEntry) => void;
  togglePin: (event: MouseEvent, id: number, isPinned: boolean) => void;
  deleteEntry: (event: MouseEvent, id: number) => void;
  handleAIAction: (id: number, content: string, actionType: string) => void;
  handleAISupplement: (id: number, input: string) => void;
  applyAIResult: (id: number) => Promise<void>;
  discardAIResult: (id: number) => void;
  copyAIText: (content: string) => Promise<void>;
  setEditingTagsId: Dispatch<SetStateAction<number | null>>;
  setEditingNoteId: Dispatch<SetStateAction<number | null>>;
  setTagInput: Dispatch<SetStateAction<string>>;
  setRevealedIds: Dispatch<SetStateAction<Set<number>>>;
  handleUpdateTags: (id: number, tags: string[]) => void;
  handleUpdateNote: (id: number, note: string) => void;
}

/** The detail pane's action bar runs the same operations a list card offers. */
export const useDetailPaneActions = ({
  items,
  focusDetailItem,
  copyToClipboard,
  openContent,
  togglePin,
  deleteEntry,
  handleAIAction,
  handleAISupplement,
  applyAIResult,
  discardAIResult,
  copyAIText,
  setEditingTagsId,
  setEditingNoteId,
  setTagInput,
  setRevealedIds,
  handleUpdateTags,
  handleUpdateNote
}: UseDetailPaneActionsOptions): DetailPaneActions =>
  useMemo(() => {
    return {
      paste: (item) =>
        void copyToClipboard(item.id, item.content, item.content_type, true, item.is_pinned, item.tags || []),
      togglePin: (event, item) => togglePin(event, item.id, item.is_pinned),
      setTagEditor: (item, open) => {
        setEditingNoteId(null);
        setTagInput("");
        setEditingTagsId(open ? item.id : null);
        if (open) focusDetailItem(item.id);
      },
      setNoteEditor: (item, open) => {
        setEditingTagsId(null);
        setTagInput("");
        setEditingNoteId(open ? item.id : null);
        if (open) focusDetailItem(item.id);
      },
      saveNote: (item, note) => {
        handleUpdateNote(item.id, note);
        setEditingNoteId(null);
      },
      addTag: (item, tag) => {
        const value = tag.trim();
        if (value && !item.tags?.includes(value)) {
          handleUpdateTags(item.id, [...(item.tags || []), value]);
        }
        setTagInput("");
        setEditingTagsId(null);
      },
      deleteTag: (item, tag) => handleUpdateTags(item.id, (item.tags || []).filter((value) => value !== tag)),
      runAI: (item, actionType) => handleAIAction(item.id, item.content, actionType),
      supplementAI: (item, input) => handleAISupplement(item.id, input),
      applyAI: (item) => void applyAIResult(item.id),
      discardAI: (item) => discardAIResult(item.id),
      copyAIText: (content) => void copyAIText(content),
      open: (item) => openContent(item),
      toggleReveal: (item) =>
        setRevealedIds((prev) => {
          const next = new Set(prev);
          if (next.has(item.id)) next.delete(item.id);
          else next.add(item.id);
          return next;
        }),
      // Move to the next row first so repeated deletes walk down the list
      // instead of jumping back to the top.
      remove: (event, item) => {
        const index = items.findIndex((entry) => entry.id === item.id);
        const neighbour = index >= 0 ? (items[index + 1] ?? items[index - 1]) : undefined;
        if (neighbour) focusDetailItem(neighbour.id);
        deleteEntry(event, item.id);
      }
    };
  }, [
    items,
    focusDetailItem,
    copyToClipboard,
    openContent,
    togglePin,
    deleteEntry,
    handleAIAction,
    handleAISupplement,
    applyAIResult,
    discardAIResult,
    copyAIText,
    setEditingTagsId,
    setEditingNoteId,
    setTagInput,
    setRevealedIds,
    handleUpdateTags,
    handleUpdateNote
  ]);
