import { useEffect, useState } from "react";

/** Window width (CSS px) at which pages switch to their two-column layout. */
export const WIDE_LAYOUT_MIN_WIDTH = 550;

const isWideNow = (minWidth: number) => typeof window !== "undefined" && window.innerWidth >= minWidth;

/** Whether the window meets a layout's width threshold (rAF-throttled). */
export const useWideWindow = (minWidth = WIDE_LAYOUT_MIN_WIDTH) => {
  const [isWide, setIsWide] = useState(() => isWideNow(minWidth));

  useEffect(() => {
    let frame = 0;
    const onResize = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => setIsWide(isWideNow(minWidth)));
    };
    onResize();
    window.addEventListener("resize", onResize);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", onResize);
    };
  }, [minWidth]);

  return isWide;
};
