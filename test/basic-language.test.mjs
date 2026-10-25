import test from 'node:test';
import assert from 'node:assert/strict';
import '../web/basic-language.js';
import { numberedTape } from './lib/basic-tape.mjs';

const evaluate = (source, host = {}) => BlipBasicLanguage.evaluate(source, {
  read: (name) => ({ X: 7, 'NAME$': 'BLIP' })[name] ?? 0,
  call: (name) => { throw Error('Unknown function ' + name); },
  ...host,
});

test('console expressions preserve precedence, unary operators, and string escaping', () => {
  for (const [source, expected] of [
    ['1+2*3', 7], ['(1+2)*3', 9], ['10-3-2', 5], ['20/2/5', 2],
    ['-X+ +3', -4], ['NOT 0 AND 3>2', 1], ['1 OR 0 AND 0', 1],
    ['"SAY ""HI"" "+NAME$', 'SAY "HI" BLIP'],
    ['STR$(INT(3.9))+CHR$(65)', '3A'], ['SIN(0)+COS(0)+SQR(9)', 4],
    ['LEN("A,B:C")', 5], ['"A"<"B"', 1],
    ['7 MOD 3', 1], ['-1 MOD 8', 7], ['1+10 MOD 4*2', 5],
    ['EXP(0)+LOG(1)+TAN(0)', 1], ['SGN(-3)+SGN(0)+SGN(2)', 0], ['FIX(-2.7)', -2],
    ['MID$("#..W..A",4,1)+MID$("#..W..A",6)', 'W.A'], ['LEFT$("BLIP",2)+RIGHT$("BLIP",1)', 'BLP'],
    ['VAL("12.5KG")+VAL("X")', 12.5], ['ASC("A")', 65], ['MID$("AB",9)+LEFT$("AB",99)', 'AB'],
  ]) assert.equal(evaluate(source), expected, source);
  assert.deepEqual(evaluate('COLOR(0.1,0.2,0.3)'), { blipColor: true, r: 0.1, g: 0.2, b: 0.3, a: 1 });
});

test('console function arguments reach the host in evaluation order', () => {
  const calls = [];
  assert.equal(evaluate('DISTANCE(X,0,WRAP(-21,0,10),4)', {
    call(name, args) {
      calls.push([name, args]);
      return name === 'WRAP' ? 9 : 5;
    },
  }), 5);
  assert.deepEqual(calls, [['WRAP', [-21, 0, 10]], ['DISTANCE', [7, 0, 9, 4]]]);
});

test('invalid console expressions fail at their syntax or arithmetic error', () => {
  for (const [source, error] of [
    ['1/0', /Division by zero/], ['(2+3', /Expected \)/],
    ['1 2', /Syntax near 2/], ['"UNCLOSED', /Syntax near/],
    ['1+', /Expression expected/], ['COLOR(1,2)', /COLOR needs/],
    ['MISSING(3)', /Unknown function MISSING/],
  ]) assert.throws(() => evaluate(source), error, source);
});

test('numbered tape parsing rejects duplicate or out-of-order lines', () => {
  assert.deepEqual([...numberedTape('10 REM START\n\n20 WAIT 0')], [
    [10, 'REM START'], [20, 'WAIT 0'],
  ]);
  assert.throws(() => numberedTape('10 GOTO 20\n10 WAIT 0'), /unique and ascending/);
  assert.throws(() => numberedTape('20 WAIT 0\n10 END'), /unique and ascending/);
  assert.throws(() => numberedTape('INVALID'), /invalid numbered tape line/);
});
