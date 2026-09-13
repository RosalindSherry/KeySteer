import assert from 'node:assert/strict'
import test from 'node:test'
import { cardPositionRatios, packedCardCenters, positionFromPoints } from './window-card-position.ts'

test('visual editing supports points, reverse drags and bounded percentage output', () => {
  assert.deepEqual(positionFromPoints({ x: .5, y: .5 }, { x: .5, y: .5 }), ['50%', '50%', '50%', '50%'])
  assert.deepEqual(positionFromPoints({ x: 1, y: 0 }, { x: 0, y: 0 }), ['0%', '0%', '100%', '0%'])
  const forward = positionFromPoints({ x: .2, y: .1 }, { x: .8, y: .9 })
  assert.deepEqual(forward, ['10%', '20%', '10%', '20%'])
  assert.deepEqual(forward, positionFromPoints({ x: .8, y: .9 }, { x: .2, y: .1 }))
  for (let n = 0; n <= 100; n++) cardPositionRatios(positionFromPoints({ x: n / 99, y: n / 103 }, { x: -.1, y: 1.1 }))
})

test('percentage insets describe a center or a top row and reject pixels', () => {
  assert.deepEqual(cardPositionRatios(), [.5, .5, .5, .5])
  assert.deepEqual(cardPositionRatios(['0%', '0%', '100%', '0%']), [0, 0, 1, 0])
  for (const input of [[50, 50, 50, 50], ['1px', '0%', '0%', '0%'], ['60%', '0%', '50%', '0%'], ['0%']]) assert.throws(() => cardPositionRatios(input))
  const positions = packedCardCenters(8, 32, 10, [0, 0, 1, 0])
  assert.equal(positions[0].y, 5)
  assert.equal(positions[2].y, 5)
  assert.equal(positions[3].x, positions[0].x)
  assert.ok(positions[3].y > positions[0].y)
})
