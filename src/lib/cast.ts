// The light takes the artwork's colour: its most saturated pixels, averaged, at a steady lightness.
// Near-grey artwork keeps pass 7's neutral blue.
export function castFrom(show: number) {
  const img = new Image();
  img.crossOrigin = 'anonymous';
  img.onload = () => {
    const c = document.createElement('canvas'); c.width = c.height = 24;
    const g = c.getContext('2d', { willReadFrequently: true })!;
    g.drawImage(img, 0, 0, 24, 24);
    const d = g.getImageData(0, 0, 24, 24).data;
    let r = 0, gr = 0, b = 0, wt = 0;
    for (let i = 0; i < d.length; i += 4) {
      const mx = Math.max(d[i], d[i + 1], d[i + 2]), mn = Math.min(d[i], d[i + 1], d[i + 2]);
      const w = (mx - mn) / 255 + 0.02; r += d[i] * w; gr += d[i + 1] * w; b += d[i + 2] * w; wt += w;
    }
    const n = d.length / 4, colourful = (wt - 0.02 * n) / n > 0.08;
    document.documentElement.style.setProperty('--cast',
      colourful ? `hsl(from rgb(${r / wt} ${gr / wt} ${b / wt}) h clamp(25, s, 60) 60)` : '#6f8fc9');
  };
  img.src = `listener://localhost/art/${show}`;
}
