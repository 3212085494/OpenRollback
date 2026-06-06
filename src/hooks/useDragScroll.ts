import { useCallback, useEffect, useRef } from "react";

interface DragScrollOptions {
  threshold?: number;
}

/**
 * 可滚动容器的拖动浏览：使用 document 级 pointer 事件，避免在子元素上失效。
 */
export function useDragScroll<T extends HTMLElement>(options: DragScrollOptions = {}) {
  const { threshold = 5 } = options;
  const ref = useRef<T>(null);
  const state = useRef({
    dragging: false,
    moved: false,
    startX: 0,
    startY: 0,
    scrollLeft: 0,
  });

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const onPointerMove = (e: PointerEvent) => {
      if (!state.current.dragging) return;

      const dx = e.pageX - state.current.startX;

      if (!state.current.moved && Math.abs(dx) > threshold) {
        state.current.moved = true;
      }

      if (state.current.moved) {
        e.preventDefault();
        el.scrollLeft = state.current.scrollLeft - dx;
      }
    };

    const endDrag = () => {
      state.current.dragging = false;
      el.style.cursor = "grab";
      el.style.userSelect = "";
    };

    const onWheel = (e: WheelEvent) => {
      const horizontal = Math.abs(e.deltaX) > Math.abs(e.deltaY);
      if (horizontal || e.shiftKey) {
        e.preventDefault();
        el.scrollLeft += horizontal ? e.deltaX : e.deltaY;
      }
    };

    document.addEventListener("pointermove", onPointerMove, { passive: false });
    document.addEventListener("pointerup", endDrag);
    document.addEventListener("pointercancel", endDrag);
    el.addEventListener("wheel", onWheel, { passive: false });

    return () => {
      document.removeEventListener("pointermove", onPointerMove);
      document.removeEventListener("pointerup", endDrag);
      document.removeEventListener("pointercancel", endDrag);
      el.removeEventListener("wheel", onWheel);
    };
  }, [threshold]);

  const onPointerDown = useCallback((e: React.PointerEvent) => {
    const el = ref.current;
    if (!el || e.button !== 0) return;

    // 回滚按钮等交互控件上不启动拖动
    const target = e.target as HTMLElement;
    if (target.closest("button")) return;

    state.current.dragging = true;
    state.current.moved = false;
    state.current.startX = e.pageX;
    state.current.scrollLeft = el.scrollLeft;
    el.style.cursor = "grabbing";
    el.style.userSelect = "none";
  }, []);

  const wasDragged = useCallback(() => {
    const result = state.current.moved;
    state.current.moved = false;
    return result;
  }, []);

  const scrollBy = useCallback((dx: number) => {
    const el = ref.current;
    if (!el) return;
    el.scrollBy({ left: dx, behavior: "smooth" });
  }, []);

  const scrollToEnd = useCallback(() => {
    const el = ref.current;
    if (!el) return;
    el.scrollTo({
      left: Math.max(0, el.scrollWidth - el.clientWidth),
      behavior: "smooth",
    });
  }, []);

  const scrollToStart = useCallback(() => {
    ref.current?.scrollTo({ left: 0, behavior: "smooth" });
  }, []);

  return {
    ref,
    dragProps: { onPointerDown },
    wasDragged,
    scrollBy,
    scrollToStart,
    scrollToEnd,
  };
}
