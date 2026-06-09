// Shared browser export helpers — used by any view that lets the user save a
// generated artifact (PRISMA flow, concept map, glossary, …) to disk.

/** Trigger a browser download for an in-memory Blob. */
export function downloadBlob(filename: string, blob: Blob) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** Rasterise an SVG string to a PNG Blob at `scale`× DPI. Pure browser APIs. */
export async function svgToPngBlob(svg: string, scale = 2): Promise<Blob> {
  // Pull width/height out of the SVG header so we know the canvas size.
  const widthMatch = svg.match(/<svg[^>]*\swidth="(\d+)"/);
  const heightMatch = svg.match(/<svg[^>]*\sheight="(\d+)"/);
  const w = widthMatch ? parseInt(widthMatch[1], 10) : 920;
  const h = heightMatch ? parseInt(heightMatch[1], 10) : 600;

  const svgBlob = new Blob([svg], { type: "image/svg+xml;charset=utf-8" });
  const url = URL.createObjectURL(svgBlob);
  try {
    const img = await new Promise<HTMLImageElement>((resolve, reject) => {
      const i = new Image();
      i.onload = () => resolve(i);
      i.onerror = reject;
      i.src = url;
    });
    const canvas = document.createElement("canvas");
    canvas.width = w * scale;
    canvas.height = h * scale;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("canvas 2d context unavailable");
    ctx.fillStyle = "#fbfaf7";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
    return await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("canvas.toBlob returned null"))), "image/png");
    });
  } finally {
    URL.revokeObjectURL(url);
  }
}
