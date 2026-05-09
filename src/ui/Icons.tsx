import type { CSSProperties, ReactNode, SVGProps } from 'react';

interface IconProps extends Omit<SVGProps<SVGSVGElement>, 'width' | 'height'> {
  size?: number;
  strokeWidth?: number;
  className?: string;
  style?: CSSProperties;
}

interface BaseProps extends IconProps {
  children: ReactNode;
}

const Base = ({ children, size = 16, strokeWidth = 1.6, className, style, ...rest }: BaseProps) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    width={size}
    height={size}
    fill="none"
    stroke="currentColor"
    strokeWidth={strokeWidth}
    strokeLinecap="round"
    strokeLinejoin="round"
    className={`dm-icon${className ? ` ${className}` : ''}`}
    style={style}
    aria-hidden="true"
    {...rest}
  >
    {children}
  </svg>
);

export const Icons = {
  Sword: (p: IconProps) => (
    <Base {...p}>
      <path d="M14.5 17.5L4 21l3.5-10.5" />
      <path d="M21 3l-9.5 9.5" />
      <path d="M21 3l-3.5 8L13 6.5L21 3z" />
      <path d="M5.5 18.5l1.5 1.5" />
    </Base>
  ),
  Wand: (p: IconProps) => (
    <Base {...p}>
      <path d="M3 21l9-9" />
      <path d="M14 4l1.2 2.4L18 8l-2.8 1.6L14 12l-1.2-2.4L10 8l2.8-1.6z" />
      <path d="M19 14l.7 1.3L21 16l-1.3.7L19 18l-.7-1.3L17 16l1.3-.7z" />
    </Base>
  ),
  Shield: (p: IconProps) => (
    <Base {...p}>
      <path d="M12 3l8 3v5c0 5-3.5 8.5-8 10c-4.5-1.5-8-5-8-10V6z" />
    </Base>
  ),
  ShieldHalf: (p: IconProps) => (
    <Base {...p}>
      <path d="M12 3l8 3v5c0 5-3.5 8.5-8 10c-4.5-1.5-8-5-8-10V6z" />
      <path d="M12 3v18" />
    </Base>
  ),
  Footprints: (p: IconProps) => (
    <Base {...p}>
      <ellipse cx="7" cy="14" rx="3" ry="4" />
      <ellipse cx="17" cy="10" rx="3" ry="4" />
      <circle cx="5" cy="20" r="1" />
      <circle cx="9" cy="21" r="1" />
      <circle cx="15" cy="16" r="1" />
      <circle cx="19" cy="17" r="1" />
    </Base>
  ),
  Run: (p: IconProps) => (
    <Base {...p}>
      <circle cx="13" cy="4" r="2" />
      <path d="M7 13l3-2 2 3l3 2l-2 5" />
      <path d="M5 17l4-2" />
      <path d="M14 8l3 2l-1 4" />
    </Base>
  ),
  Hand: (p: IconProps) => (
    <Base {...p}>
      <path d="M9 11V5a1.5 1.5 0 113 0v5" />
      <path d="M12 10V4a1.5 1.5 0 113 0v6" />
      <path d="M15 10V6a1.5 1.5 0 113 0v8c0 4-2 7-6 7s-6-3-6-7v-3a1.5 1.5 0 113 0v2" />
    </Base>
  ),
  ArrowReverse: (p: IconProps) => (
    <Base {...p}>
      <path d="M9 14l-4-4l4-4" />
      <path d="M5 10h10a4 4 0 014 4v3" />
    </Base>
  ),
  Hourglass: (p: IconProps) => (
    <Base {...p}>
      <path d="M7 3h10" />
      <path d="M7 21h10" />
      <path d="M7 3c0 5 5 6 5 9s-5 4-5 9" />
      <path d="M17 3c0 5-5 6-5 9s5 4 5 9" />
    </Base>
  ),
  Dice: (p: IconProps) => (
    <Base {...p}>
      <path d="M3 7l9-4l9 4l-9 4z" />
      <path d="M3 7v10l9 4" />
      <path d="M21 7v10l-9 4" />
      <circle cx="12" cy="9" r="0.5" fill="currentColor" />
      <circle cx="7" cy="13" r="0.5" fill="currentColor" />
      <circle cx="17" cy="13" r="0.5" fill="currentColor" />
    </Base>
  ),
  D20: (p: IconProps) => (
    <Base {...p}>
      <path d="M12 3l8 5v8l-8 5l-8-5V8z" />
      <path d="M12 3v18" />
      <path d="M4 8l16 8" />
      <path d="M20 8L4 16" />
    </Base>
  ),
  Settings: (p: IconProps) => (
    <Base {...p}>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.7 1.7 0 00.3 1.8l.1.1a2 2 0 11-2.8 2.8l-.1-.1a1.7 1.7 0 00-1.8-.3a1.7 1.7 0 00-1 1.5V21a2 2 0 11-4 0v-.1a1.7 1.7 0 00-1.1-1.5a1.7 1.7 0 00-1.8.3l-.1.1a2 2 0 11-2.8-2.8l.1-.1a1.7 1.7 0 00.3-1.8a1.7 1.7 0 00-1.5-1H3a2 2 0 110-4h.1a1.7 1.7 0 001.5-1.1a1.7 1.7 0 00-.3-1.8l-.1-.1a2 2 0 112.8-2.8l.1.1a1.7 1.7 0 001.8.3H9a1.7 1.7 0 001-1.5V3a2 2 0 114 0v.1a1.7 1.7 0 001 1.5a1.7 1.7 0 001.8-.3l.1-.1a2 2 0 112.8 2.8l-.1.1a1.7 1.7 0 00-.3 1.8V9a1.7 1.7 0 001.5 1H21a2 2 0 110 4h-.1a1.7 1.7 0 00-1.5 1z" />
    </Base>
  ),
  X: (p: IconProps) => (
    <Base {...p}>
      <path d="M18 6L6 18" />
      <path d="M6 6l12 12" />
    </Base>
  ),
  Plus: (p: IconProps) => (
    <Base {...p}>
      <path d="M12 5v14" />
      <path d="M5 12h14" />
    </Base>
  ),
  Minus: (p: IconProps) => (
    <Base {...p}>
      <path d="M5 12h14" />
    </Base>
  ),
  Send: (p: IconProps) => (
    <Base {...p}>
      <path d="M22 2L11 13" />
      <path d="M22 2l-7 20l-4-9l-9-4z" />
    </Base>
  ),
  Stop: (p: IconProps) => (
    <Base {...p}>
      <rect x="6" y="6" width="12" height="12" rx="1" />
    </Base>
  ),
  Eye: (p: IconProps) => (
    <Base {...p}>
      <path d="M2 12s3.5-7 10-7s10 7 10 7s-3.5 7-10 7s-10-7-10-7z" />
      <circle cx="12" cy="12" r="3" />
    </Base>
  ),
  EyeOff: (p: IconProps) => (
    <Base {...p}>
      <path d="M9.9 4.2A10 10 0 0112 4c6.5 0 10 7 10 7a17 17 0 01-3.2 4.1" />
      <path d="M6.6 6.6A17 17 0 002 11s3.5 7 10 7c2 0 3.7-.6 5.3-1.6" />
      <path d="M9.9 9.9a3 3 0 004.2 4.2" />
      <path d="M2 2l20 20" />
    </Base>
  ),
  ChevronRight: (p: IconProps) => (
    <Base {...p}>
      <path d="M9 6l6 6l-6 6" />
    </Base>
  ),
  ChevronLeft: (p: IconProps) => (
    <Base {...p}>
      <path d="M15 6l-6 6l6 6" />
    </Base>
  ),
  ChevronDown: (p: IconProps) => (
    <Base {...p}>
      <path d="M6 9l6 6l6-6" />
    </Base>
  ),
  Check: (p: IconProps) => (
    <Base {...p}>
      <path d="M5 12l5 5L20 7" />
    </Base>
  ),
  Warning: (p: IconProps) => (
    <Base {...p}>
      <path d="M10.3 3.9L2 18.5A2 2 0 003.7 21.5h16.6a2 2 0 001.7-3L13.7 3.9a2 2 0 00-3.4 0z" />
      <path d="M12 9v4" />
      <circle cx="12" cy="17" r="0.6" fill="currentColor" />
    </Base>
  ),
  Info: (p: IconProps) => (
    <Base {...p}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 8h.01" />
      <path d="M11 12h1v4h1" />
    </Base>
  ),
  Sparkle: (p: IconProps) => (
    <Base {...p}>
      <path d="M12 3l1.7 4.3L18 9l-4.3 1.7L12 15l-1.7-4.3L6 9l4.3-1.7z" />
      <path d="M19 16l.7 1.5L21 18l-1.3.5L19 20l-.7-1.5L17 18l1.3-.5z" />
    </Base>
  ),
  Cloud: (p: IconProps) => (
    <Base {...p}>
      <path d="M17 18a4 4 0 000-8a6 6 0 00-11.7 1.4A4 4 0 006 19h11" />
    </Base>
  ),
  Server: (p: IconProps) => (
    <Base {...p}>
      <rect x="3" y="4" width="18" height="7" rx="1.5" />
      <rect x="3" y="13" width="18" height="7" rx="1.5" />
      <circle cx="7" cy="7.5" r="0.6" fill="currentColor" />
      <circle cx="7" cy="16.5" r="0.6" fill="currentColor" />
    </Base>
  ),
  Cpu: (p: IconProps) => (
    <Base {...p}>
      <rect x="5" y="5" width="14" height="14" rx="1.5" />
      <rect x="9" y="9" width="6" height="6" rx="0.8" />
      <path d="M9 2v3" />
      <path d="M15 2v3" />
      <path d="M9 19v3" />
      <path d="M15 19v3" />
      <path d="M2 9h3" />
      <path d="M2 15h3" />
      <path d="M19 9h3" />
      <path d="M19 15h3" />
    </Base>
  ),
  Globe: (p: IconProps) => (
    <Base {...p}>
      <circle cx="12" cy="12" r="9" />
      <path d="M3 12h18" />
      <path d="M12 3a13 13 0 010 18" />
      <path d="M12 3a13 13 0 000 18" />
    </Base>
  ),
  Save: (p: IconProps) => (
    <Base {...p}>
      <path d="M5 4h11l4 4v12a1 1 0 01-1 1H5a1 1 0 01-1-1V5a1 1 0 011-1z" />
      <path d="M8 4v5h7V4" />
      <path d="M8 14h8v7H8z" />
    </Base>
  ),
  Heart: (p: IconProps) => (
    <Base {...p}>
      <path d="M12 20s-7-4.5-7-10a4 4 0 017-2.6A4 4 0 0119 10c0 5.5-7 10-7 10z" />
    </Base>
  ),
  Skull: (p: IconProps) => (
    <Base {...p}>
      <path d="M12 3a8 8 0 00-8 8v3l2 2v3h2v-2h2v2h4v-2h2v2h2v-3l2-2v-3a8 8 0 00-8-8z" />
      <circle cx="9" cy="11" r="1.2" />
      <circle cx="15" cy="11" r="1.2" />
      <path d="M11 16h2" />
    </Base>
  ),
  Crosshair: (p: IconProps) => (
    <Base {...p}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 2v4" />
      <path d="M12 18v4" />
      <path d="M2 12h4" />
      <path d="M18 12h4" />
      <circle cx="12" cy="12" r="2" />
    </Base>
  ),
  Zap: (p: IconProps) => (
    <Base {...p}>
      <path d="M13 2L4 14h7l-1 8l9-12h-7z" />
    </Base>
  ),
  Flame: (p: IconProps) => (
    <Base {...p}>
      <path d="M12 22a7 7 0 007-7c0-3-1-5-3-8c0 0-1 3-3 3s-2-3-2-6c-3 3-6 6-6 11a7 7 0 007 7z" />
      <path d="M9 16a3 3 0 003 3a3 3 0 003-3" />
    </Base>
  ),
  Snowflake: (p: IconProps) => (
    <Base {...p}>
      <path d="M12 2v20" />
      <path d="M2 12h20" />
      <path d="M5 5l14 14" />
      <path d="M19 5L5 19" />
    </Base>
  ),
  Moon: (p: IconProps) => (
    <Base {...p}>
      <path d="M21 12.8A9 9 0 1111.2 3a7 7 0 009.8 9.8z" />
    </Base>
  ),
  Compass: (p: IconProps) => (
    <Base {...p}>
      <circle cx="12" cy="12" r="9" />
      <path d="M16 8l-2 6l-6 2l2-6z" />
    </Base>
  ),
  Map: (p: IconProps) => (
    <Base {...p}>
      <path d="M9 3L3 5v16l6-2l6 2l6-2V3l-6 2z" />
      <path d="M9 3v16" />
      <path d="M15 5v16" />
    </Base>
  ),
  Layers: (p: IconProps) => (
    <Base {...p}>
      <path d="M12 3l9 5l-9 5l-9-5z" />
      <path d="M3 13l9 5l9-5" />
      <path d="M3 18l9 5l9-5" />
    </Base>
  ),
  Ruler: (p: IconProps) => (
    <Base {...p}>
      <path d="M3 17l4 4L21 7l-4-4z" />
      <path d="M7 11l2 2" />
      <path d="M10 8l2 2" />
      <path d="M13 5l2 2" />
      <path d="M4 14l2 2" />
    </Base>
  ),
  ZoomIn: (p: IconProps) => (
    <Base {...p}>
      <circle cx="11" cy="11" r="7" />
      <path d="M21 21l-4.5-4.5" />
      <path d="M11 8v6" />
      <path d="M8 11h6" />
    </Base>
  ),
  ZoomOut: (p: IconProps) => (
    <Base {...p}>
      <circle cx="11" cy="11" r="7" />
      <path d="M21 21l-4.5-4.5" />
      <path d="M8 11h6" />
    </Base>
  ),
  Maximize: (p: IconProps) => (
    <Base {...p}>
      <path d="M4 9V4h5" />
      <path d="M20 9V4h-5" />
      <path d="M4 15v5h5" />
      <path d="M20 15v5h-5" />
    </Base>
  ),
  GridIcon: (p: IconProps) => (
    <Base {...p}>
      <rect x="4" y="4" width="7" height="7" />
      <rect x="13" y="4" width="7" height="7" />
      <rect x="4" y="13" width="7" height="7" />
      <rect x="13" y="13" width="7" height="7" />
    </Base>
  ),
  User: (p: IconProps) => (
    <Base {...p}>
      <circle cx="12" cy="8" r="4" />
      <path d="M4 21c0-4.4 3.6-8 8-8s8 3.6 8 8" />
    </Base>
  ),
  Image: (p: IconProps) => (
    <Base {...p}>
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <circle cx="9" cy="9" r="1.5" />
      <path d="M21 15l-5-5L6 21" />
    </Base>
  ),
  Refresh: (p: IconProps) => (
    <Base {...p}>
      <path d="M21 12a9 9 0 01-15 6.7L3 16" />
      <path d="M3 12a9 9 0 0115-6.7L21 8" />
      <path d="M21 3v5h-5" />
      <path d="M3 21v-5h5" />
    </Base>
  ),
  Minimize: (p: IconProps) => (
    <Base {...p}>
      <path d="M5 12h14" />
    </Base>
  ),
  Square: (p: IconProps) => (
    <Base {...p}>
      <rect x="5" y="5" width="14" height="14" rx="1" />
    </Base>
  ),
  Folder: (p: IconProps) => (
    <Base {...p}>
      <path d="M3 6a1 1 0 011-1h5l2 2h9a1 1 0 011 1v10a1 1 0 01-1 1H4a1 1 0 01-1-1z" />
    </Base>
  ),
  Lock: (p: IconProps) => (
    <Base {...p}>
      <rect x="4" y="11" width="16" height="10" rx="2" />
      <path d="M8 11V7a4 4 0 018 0v4" />
    </Base>
  ),
  Book: (p: IconProps) => (
    <Base {...p}>
      <path d="M4 4a2 2 0 012-2h13v18H6a2 2 0 00-2 2" />
      <path d="M4 4v18" />
    </Base>
  ),
  Crown: (p: IconProps) => (
    <Base {...p}>
      <path d="M3 18l2-10l5 4l2-7l2 7l5-4l2 10z" />
      <path d="M3 21h18" />
    </Base>
  ),
  Bow: (p: IconProps) => (
    <Base {...p}>
      <path d="M5 19c8 0 14-6 14-14" />
      <path d="M5 19l4-1l-1 4" />
      <path d="M5 19l14-14" />
      <path d="M16 5h3v3" />
    </Base>
  ),
  Potion: (p: IconProps) => (
    <Base {...p}>
      <path d="M9 3h6" />
      <path d="M10 3v5l-3 6a4 4 0 004 7h2a4 4 0 004-7l-3-6V3" />
      <path d="M7.5 14h9" />
    </Base>
  ),
  Scroll: (p: IconProps) => (
    <Base {...p}>
      <path d="M19 17V5a2 2 0 00-2-2H6" />
      <path d="M3 5v12a3 3 0 003 3h12a3 3 0 003-3v-1H8a1 1 0 00-1 1" />
      <path d="M3 5a2 2 0 014 0v12" />
    </Base>
  ),
  Star: (p: IconProps) => (
    <Base {...p}>
      <path d="M12 3l2.7 5.5l6.1.9l-4.4 4.3l1 6.1L12 17l-5.4 2.8l1-6.1L3.2 9.4l6.1-.9z" />
    </Base>
  ),
  Volume: (p: IconProps) => (
    <Base {...p}>
      <path d="M11 5L6 9H3v6h3l5 4z" />
      <path d="M19 12a4 4 0 01-2 3.5" />
      <path d="M22 12a7 7 0 01-3 5.7" />
    </Base>
  ),
  Tag: (p: IconProps) => (
    <Base {...p}>
      <path d="M3 3h8l10 10l-8 8L3 11z" />
      <circle cx="7.5" cy="7.5" r="1.2" />
    </Base>
  ),
  Coin: (p: IconProps) => (
    <Base {...p}>
      <circle cx="12" cy="12" r="9" />
      <path d="M9 9h4a2 2 0 010 4h-3a2 2 0 000 4h5" />
      <path d="M12 7v2" />
      <path d="M12 15v2" />
    </Base>
  ),
} as const;

export type IconName = keyof typeof Icons;
