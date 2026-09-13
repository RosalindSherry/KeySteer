export function positionFromPoints(start: { x: number; y: number }, end: { x: number; y: number }): string[] {
  const percent = (value: number) => Math.round(Math.max(0, Math.min(1, value)) * 1000) / 10
  const left = Math.min(percent(start.x), percent(end.x)), right = Math.max(percent(start.x), percent(end.x))
  const top = Math.min(percent(start.y), percent(end.y)), bottom = Math.max(percent(start.y), percent(end.y))
  return [top, Math.round((100 - right) * 10) / 10, Math.round((100 - bottom) * 10) / 10, left].map(value => `${value}%`)
}

export function cardPositionRatios(value: unknown = ['50%', '50%', '50%', '50%']): number[] {
  if (!Array.isArray(value) || value.length !== 4) throw new Error('window.card.position requires four percentages')
  const ratios = value.map(part => {
    if (typeof part !== 'string' || !/^\s*(?:\d+(?:\.\d*)?|\.\d+)%\s*$/.test(part)) throw new Error('window.card.position requires percentages')
    const ratio = Number(part.trim().slice(0, -1)) / 100
    if (!Number.isFinite(ratio) || ratio < 0 || ratio > 1) throw new Error('window.card.position must be 0%..100%')
    return ratio
  })
  if (ratios[0] + ratios[2] > 1 + 1e-12 || ratios[1] + ratios[3] > 1 + 1e-12) throw new Error('window.card.position opposite sides exceed 100%')
  return ratios
}

// Values are relative to the simulator's current screen, never a previous screen.
export function packedCardCenters(count: number, width: number, height: number, insets: number[]): Array<{ x: number; y: number }> {
  if (!count) return []
  const [top, right, bottom, left] = insets
  const gap = 1
  const availableRows = Math.max(1, Math.floor((100 + gap) / (height + gap)))
  const minimum = Math.ceil(count / availableRows)
  width = Math.min(width, Math.max(1, (100 - gap * (minimum - 1)) / minimum))
  const capacity = Math.max(1, Math.floor((100 + gap) / (width + gap)))
  const regionWidth = Math.max(0, 1 - left - right) * 100
  const regionHeight = Math.max(0, 1 - top - bottom) * 100
  let columns = minimum
  if (regionWidth > .001) columns = Math.max(minimum, Math.min(count, Math.max(1, Math.floor((regionWidth + gap) / (width + gap)))))
  else {
    let best = Infinity
    for (let n = minimum; n <= Math.max(minimum, Math.min(capacity, count)); n++) {
      const cost = n * (width + gap) + Math.ceil(count / n) * (height + gap)
      if (cost < best) { best = cost; columns = n }
    }
  }
  const totalWidth = columns * (width + gap) - gap
  const totalHeight = Math.ceil(count / columns) * (height + gap) - gap
  const x = Math.max(0, Math.min(Math.max(0, 100 - totalWidth), left * 100 - (regionWidth > .001 ? 0 : totalWidth / 2)))
  const y = Math.max(0, Math.min(Math.max(0, 100 - totalHeight), top * 100 + regionHeight / 2 - totalHeight / 2))
  return Array.from({ length: count }, (_, i) => ({ x: x + (i % columns) * (width + gap) + width / 2, y: y + Math.floor(i / columns) * (height + gap) + height / 2 }))
}
