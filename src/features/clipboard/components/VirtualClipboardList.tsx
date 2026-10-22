import React, { useRef, useImperativeHandle, useCallback, useMemo } from 'react';
import { Virtuoso, VirtuosoHandle } from 'react-virtuoso';
import type { ListRange } from 'react-virtuoso';
import type { ClipboardEntry } from "../../../shared/types";
import { isPointerPick } from "../../../shared/lib/pointerPick";
import type { VirtualClipboardListHandle, VirtualClipboardListProps } from "../types";

type VirtuosoListContext = {
    header?: React.ReactNode;
    hasMore: boolean;
    isLoading: boolean;
    loadingLabel: string;
    loadMoreLabel: string;
};

const ListHeader = ({ context }: { context?: VirtuosoListContext }) => {
    const header = context?.header;
    return header ? <div className="list-header">{header}</div> : null;
};

const ListFooter = ({ context }: { context?: VirtuosoListContext }) => {
    if (!context) return null;
    const { isLoading, hasMore } = context;
    if (!isLoading && !hasMore) return null;

    return (
        <div style={{
            padding: '20px',
            textAlign: 'center',
            opacity: 0.6,
            fontSize: 'var(--font-size-sm)',
            color: 'var(--text-secondary)'
        }}>
            <span role="status" aria-live="polite">
                {isLoading ? context.loadingLabel : context.loadMoreLabel}
            </span>
        </div>
    );
};

const VirtualClipboardList = React.forwardRef<VirtualClipboardListHandle, VirtualClipboardListProps>(
    (props, ref) => {
        const {
            items,
            renderItem,
            onLoadMore,
            hasMore,
            isLoading,
            selectedIndex,
            isKeyboardMode,
            onScroll,
            compactMode,
            header,
            loadingLabel,
            loadMoreLabel
        } = props;

        const virtuosoRef = useRef<VirtuosoHandle>(null);
        const visibleRangeRef = useRef<ListRange | null>(null);
        // Read by the selection effect without re-running it on list updates.
        const itemsRef = useRef(items);
        itemsRef.current = items;
        useImperativeHandle(ref, () => ({
            scrollToItem: (index: number) => {
                virtuosoRef.current?.scrollIntoView({
                    index,
                    behavior: 'smooth',
                    align: 'center',
                });
            },
            scrollToTop: () => {
                virtuosoRef.current?.scrollTo({
                    top: 0,
                    behavior: 'auto'
                });
            },
            resetAfterIndex: (_index: number) => {
                // Not needed with Virtuoso as it handles dynamic heights automatically
            }
        }));

        // Keep keyboard selection visible even when the item is only in overscan
        React.useEffect(() => {
            if (!isKeyboardMode || selectedIndex < 0) return;
            if (isPointerPick(itemsRef.current[selectedIndex]?.id)) return;

            const range = visibleRangeRef.current;
            const edgeBuffer = 1;

            if (!range) {
                virtuosoRef.current?.scrollToIndex({
                    index: selectedIndex,
                    behavior: 'auto',
                    align: 'center',
                });
                return;
            }

            if (selectedIndex < range.startIndex + edgeBuffer) {
                virtuosoRef.current?.scrollToIndex({
                    index: selectedIndex,
                    behavior: 'auto',
                    align: 'start',
                });
                return;
            }

            if (selectedIndex > range.endIndex - edgeBuffer) {
                virtuosoRef.current?.scrollToIndex({
                    index: selectedIndex,
                    behavior: 'auto',
                    align: 'end',
                });
            }
        }, [selectedIndex, isKeyboardMode]);


        // Handle scroll events
        const handleScroll = useCallback((scrollTop: number) => {
            onScroll?.(scrollTop);
        }, [onScroll]);

        // Handle end reached for infinite loading
        const handleEndReached = useCallback(() => {
            if (hasMore && !isLoading && onLoadMore) {
                onLoadMore();
            }
        }, [hasMore, isLoading, onLoadMore]);

        const handleRangeChanged = useCallback((range: ListRange) => {
            visibleRangeRef.current = range;
        }, []);

        // Memoized item renderer for Virtuoso
        const itemContent = useCallback((index: number, item: ClipboardEntry) => {
            return (
                <div style={{ paddingBottom: compactMode ? 2 : 4 }}>
                    {renderItem(item, index, index === 0)}
                </div>
            );
        }, [renderItem, compactMode]);

        const computeItemKey = useCallback((_index: number, item: ClipboardEntry) => item.id, []);

        const components = useMemo(() => ({
            Header: ListHeader,
            Footer: ListFooter
        }), []);

        const context = useMemo(() => ({
            header,
            hasMore,
            isLoading,
            loadingLabel,
            loadMoreLabel
        }), [header, hasMore, isLoading, loadingLabel, loadMoreLabel]);

        return (
            <div className="virtual-list-wrapper" style={{ height: '100%', width: '100%' }}>
                <Virtuoso
                    ref={virtuosoRef}
                    data={items}
                    // Key rows by entry id, not index: a new copy shifts every
                    // index, which otherwise remounts every visible card.
                    computeItemKey={computeItemKey}
                    itemContent={itemContent}
                    components={components}
                    context={context}
                    style={{ height: '100%' }}
                    onScroll={(e) => handleScroll((e.currentTarget as HTMLElement).scrollTop)}
                    endReached={handleEndReached}
                    rangeChanged={handleRangeChanged}
                    overscan={400} // Pre-render 400px of content for smoother scrolling
                />
            </div>
        );
    }
);

VirtualClipboardList.displayName = 'VirtualClipboardList';

export { VirtualClipboardList };
export default VirtualClipboardList;
