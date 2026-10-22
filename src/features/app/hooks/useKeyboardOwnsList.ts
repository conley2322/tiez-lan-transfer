import { useCallback, useEffect, useRef } from "react";
import { isPointerPick } from "../../../shared/lib/pointerPick";

/** Pointer travel (px) that counts as the user moving the mouse. */
const POINTER_MOVE_SLOP = 3;
/** Body class while the arrow keys drive the list; rows drop their hover look. */
const KEYBOARD_OWNS_LIST_CLASS = "keyboard-owns-list";

type Point = { x: number; y: number };

/**
 * While the arrow keys drive the list, rows scrolling under a pointer that
 * stays put still get hovered (and fire mouse events); only a pointer that
 * really moves hands the list back to the mouse. The same holds when the
 * window opens or comes back under a resting cursor. Returns `notePointer`,
 * which records a pointer position and is true once the pointer may act.
 */
export const useKeyboardOwnsList = (isKeyboardMode: boolean, selectedItemId: number | null) => {
  const pointerRef = useRef<Point | null>(null);
  // `anchor` is where the pointer was when the keys took over (null until a
  // pointer event tells us).
  const ownsRef = useRef<{ anchor: Point | null } | null>(null);

  const setOwner = useCallback((owner: { anchor: Point | null } | null) => {
    ownsRef.current = owner;
    document.body.classList.toggle(KEYBOARD_OWNS_LIST_CLASS, owner !== null);
  }, []);

  const notePointer = useCallback((x: number, y: number) => {
    pointerRef.current = { x, y };
    const owned = ownsRef.current;
    if (!owned) return true;
    if (!owned.anchor) {
      owned.anchor = { x, y };
      return false;
    }
    if (Math.abs(x - owned.anchor.x) > POINTER_MOVE_SLOP || Math.abs(y - owned.anchor.y) > POINTER_MOVE_SLOP) {
      setOwner(null);
      return true;
    }
    return false;
  }, [setOwner]);

  useEffect(() => {
    const onMove = (event: MouseEvent) => {
      notePointer(event.screenX, event.screenY);
    };
    window.addEventListener("mousemove", onMove, { passive: true });
    return () => window.removeEventListener("mousemove", onMove);
  }, [notePointer]);

  useEffect(() => {
    if (!isKeyboardMode) {
      setOwner(null);
      return;
    }
    // The mouse picking a row doesn't hand the list to the keys.
    if (selectedItemId != null && !isPointerPick(selectedItemId)) setOwner({ anchor: pointerRef.current });
  }, [isKeyboardMode, selectedItemId, setOwner]);

  // Opening under the cursor synthesizes pointer events at a spot the user
  // never pointed at; wait for a real move.
  useEffect(() => {
    const hold = () => {
      if (document.visibilityState === "visible") setOwner({ anchor: null });
    };
    window.addEventListener("focus", hold);
    document.addEventListener("visibilitychange", hold);
    return () => {
      window.removeEventListener("focus", hold);
      document.removeEventListener("visibilitychange", hold);
    };
  }, [setOwner]);

  useEffect(() => () => setOwner(null), [setOwner]);

  return notePointer;
};
