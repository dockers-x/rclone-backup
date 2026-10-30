/**
 * The goo filter: blur the shapes together, then cut the alpha back to a hard edge.
 * `crisp` lays the unblurred shapes back on top, so only the gaps between them go gooey.
 */
export function GooFilter({
  id,
  blur = 9,
  crisp = false,
}: {
  id: string;
  blur?: number;
  crisp?: boolean;
}) {
  return (
    <svg aria-hidden width="0" height="0" className="absolute">
      <defs>
        {/* The region leaves room for the blur and no more, since all of it gets filtered. sRGB, or the
            blur blends in linear light and leaves a ring where the shapes overlap. The alpha cut is
            tuned so a tab-sized pill comes out of the goo the same size it went in, letting goo and
            plain shapes trade places without a jump. */}
        <filter
          id={id}
          x="-10%"
          y="-40%"
          width="120%"
          height="180%"
          colorInterpolationFilters="sRGB"
        >
          <feGaussianBlur in="SourceGraphic" stdDeviation={blur} result="b" />
          <feColorMatrix
            in="b"
            mode="matrix"
            values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 20 -9.1"
            result="goo"
          />
          {crisp && (
            <feComposite in="SourceGraphic" in2="goo" operator="atop" />
          )}
        </filter>
      </defs>
    </svg>
  );
}
