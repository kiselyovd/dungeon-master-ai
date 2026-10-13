import { type ImgHTMLAttributes, useState } from 'react';

interface Props
  extends Omit<ImgHTMLAttributes<HTMLImageElement>, 'src' | 'alt' | 'onError' | 'onLoad'> {
  src: string;
  assetId: string;
}

/** Optional atmosphere never replaces a label or blocks the owning action. */
export function DecorativeImage({ src, assetId, style, ...props }: Props) {
  const [failedSrc, setFailedSrc] = useState<string | null>(null);
  return (
    <img
      {...props}
      src={src}
      alt=""
      data-asset-id={assetId}
      style={{ ...style, visibility: failedSrc === src ? 'hidden' : style?.visibility }}
      onError={() => {
        setFailedSrc(src);
        console.warn('[art:WARN] decorative_asset_unavailable', { assetId });
      }}
      onLoad={(event) => {
        setFailedSrc(null);
        console.info('[art:INFO] decorative_asset_loaded', {
          assetId,
          width: event.currentTarget.naturalWidth,
          height: event.currentTarget.naturalHeight,
        });
      }}
    />
  );
}
