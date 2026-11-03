// "Fit to window": the pure part. Given the bounding box of everything drawn (in document units) and the size of
// the viewport (in pixels) it returns the scale and the document point shown at the viewport's top-left corner.

export const MIN_ZOOM = 0.1;
export const MAX_ZOOM = 4;
export const FIT_MARGIN = 24; // pixels of white space around the automaton

const clamp = (value, low, high) => Math.min(high, Math.max(low, value));

/**
 * @param {{minX: number, minY: number, maxX: number, maxY: number} | null} box  null/undefined = nothing drawn
 * @param {{width: number, height: number}} viewport
 * @param {{margin?: number, minZoom?: number, maxZoom?: number}} options  `maxZoom` is 1: small automata stay at 100%
 * @returns {{scale: number, offsetX: number, offsetY: number}}  offset = document point at the viewport's top-left
 */
export function fitView(box, viewport, { margin = FIT_MARGIN, minZoom = MIN_ZOOM, maxZoom = 1 } = {}) {
  if (!box) return { scale: 1, offsetX: 0, offsetY: 0 };
  const boxW = box.maxX - box.minX;
  const boxH = box.maxY - box.minY;
  const room = (size, extent) => (extent > 0 ? (size - 2 * margin) / extent : Infinity);
  const wanted = Math.min(room(viewport.width, boxW), room(viewport.height, boxH), maxZoom);
  const scale = clamp(wanted, minZoom, maxZoom);
  // center on each axis; if the box still overflows (minimum zoom) anchor its start at the margin
  const place = (min, extent, size) => {
    const visible = size / scale;
    return visible >= extent + (2 * margin) / scale ? min - (visible - extent) / 2 : min - margin / scale;
  };
  return { scale, offsetX: place(box.minX, boxW, viewport.width), offsetY: place(box.minY, boxH, viewport.height) };
}
