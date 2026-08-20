import { useCallback, useState } from 'react';
import { cn } from '@/shared/lib/utils';
import { ProjectTerminalPanelContainer } from './ProjectTerminalPanelContainer';

interface ProjectTerminalDrawerProps {
  onClose: () => void;
}

const MIN_HEIGHT = 160;
const MAX_HEIGHT = 800;
const DEFAULT_HEIGHT = 320;

export function ProjectTerminalDrawer({ onClose }: ProjectTerminalDrawerProps) {
  const [height, setHeight] = useState(DEFAULT_HEIGHT);
  const [isDragging, setIsDragging] = useState(false);

  const startResize = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      const startY = e.clientY;
      const startHeight = height;
      setIsDragging(true);

      const onMouseMove = (e: MouseEvent) => {
        const delta = startY - e.clientY;
        setHeight(
          Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, startHeight + delta))
        );
      };

      const onMouseUp = () => {
        setIsDragging(false);
        window.removeEventListener('mousemove', onMouseMove);
        window.removeEventListener('mouseup', onMouseUp);
      };

      window.addEventListener('mousemove', onMouseMove);
      window.addEventListener('mouseup', onMouseUp);
    },
    [height]
  );

  return (
    <div
      className="fixed inset-x-0 bottom-0 z-50 flex flex-col border-t border-border bg-secondary shadow-lg"
      style={{ height }}
    >
      {/* Resize handle at the top edge of the bottom drawer. */}
      <div
        className={cn(
          'flex items-center justify-center py-0.5 cursor-ns-resize bg-panel border-b border-border',
          isDragging && 'bg-brand/10'
        )}
        onMouseDown={startResize}
      >
        <div className="w-8 h-0.5 rounded-full bg-low/50" />
      </div>

      <div className="min-h-0 flex-1">
        <ProjectTerminalPanelContainer onClose={onClose} />
      </div>
    </div>
  );
}
