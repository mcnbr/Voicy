import { useEffect, useRef } from 'react';

interface AudioWaveformProps {
  isPlaying: boolean;
  onPlayEnd?: () => void;
}

export function AudioWaveform({ isPlaying, onPlayEnd }: AudioWaveformProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const frameRef = useRef<number>(0);

  useEffect(() => {
    if (!canvasRef.current) return;
    const ctx = canvasRef.current.getContext('2d');
    if (!ctx) return;

    let stopped = false;
    let startTime = 0;

    const draw = (timestamp: number) => {
      if (stopped || !canvasRef.current) return;
      const canvas = canvasRef.current;
      const { width, height } = canvas;
      ctx.clearRect(0, 0, width, height);

      if (!isPlaying) {
        const data = new Array(32).fill(0.15);
        const barWidth = Math.max(2, (width / 32) - 2);
        const gap = 2;
        const centerY = height / 2;

        for (let i = 0; i < 32; i++) {
          const level = data[i];
          const barHeight = Math.max(1, level * (height * 0.4));
          const x = i * (barWidth + gap);
          ctx.fillStyle = '#2ECA7F';
          ctx.globalAlpha = 0.2;
          ctx.beginPath();
          ctx.roundRect(x, centerY - barHeight, barWidth, barHeight, 2);
          ctx.fill();
          ctx.globalAlpha = 0.1;
          ctx.beginPath();
          ctx.roundRect(x, centerY, barWidth, barHeight, 2);
          ctx.fill();
        }
        ctx.globalAlpha = 1;
        return;
      }

      if (startTime === 0) startTime = timestamp;
      const elapsed = timestamp - startTime;

      if (elapsed > 3000) {
        stopped = true;
        onPlayEnd?.();
        return;
      }

      const data = Array.from({ length: 32 }, () => Math.random() * 0.6 + 0.2);
      const barWidth = Math.max(2, (width / 32) - 2);
      const gap = 2;
      const centerY = height / 2;

      for (let i = 0; i < 32; i++) {
        const level = data[i];
        const barHeight = Math.max(1, level * (height * 0.4));
        const x = i * (barWidth + gap);
        ctx.fillStyle = '#2ECA7F';
        ctx.globalAlpha = 0.6 + level * 0.3;
        ctx.beginPath();
        ctx.roundRect(x, centerY - barHeight, barWidth, barHeight, 2);
        ctx.fill();
        ctx.globalAlpha = 0.3 + level * 0.2;
        ctx.beginPath();
        ctx.roundRect(x, centerY, barWidth, barHeight, 2);
        ctx.fill();
      }
      ctx.globalAlpha = 1;
      frameRef.current = requestAnimationFrame(draw);
    };

    frameRef.current = requestAnimationFrame(draw);
    return () => {
      cancelAnimationFrame(frameRef.current);
      stopped = true;
    };
  }, [isPlaying, onPlayEnd]);

  return (
    <canvas
      ref={canvasRef}
      width={280}
      height={40}
      style={{ flex: 1, height: '40px', borderRadius: '4px' }}
    />
  );
}
