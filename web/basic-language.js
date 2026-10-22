/* Expression syntax for the text console. Game functions come from the host. */
(function (root) {
  'use strict';

  function divide(a, b) {
    if (Number(b) === 0) throw Error('Division by zero');
    return Number(a) / Number(b);
  }

  const operators = {
    OR:   { precedence: 1, apply: (a, b) => Number(Boolean(a) || Boolean(b)) },
    AND:  { precedence: 2, apply: (a, b) => Number(Boolean(a) && Boolean(b)) },
    '=':  { precedence: 3, apply: (a, b) => Number(a === b) },
    '<>': { precedence: 3, apply: (a, b) => Number(a !== b) },
    '<':  { precedence: 3, apply: (a, b) => Number(a < b) },
    '>':  { precedence: 3, apply: (a, b) => Number(a > b) },
    '<=': { precedence: 3, apply: (a, b) => Number(a <= b) },
    '>=': { precedence: 3, apply: (a, b) => Number(a >= b) },
    '+':  { precedence: 4, apply: (a, b) => typeof a === 'string' || typeof b === 'string'
      ? String(a) + String(b) : Number(a) + Number(b) },
    '-':  { precedence: 4, apply: (a, b) => Number(a) - Number(b) },
    '*':  { precedence: 5, apply: (a, b) => Number(a) * Number(b) },
    '/':  { precedence: 5, apply: divide },
    // Floored, like the Rust runtime: -1 MOD 8 is 7, so wrapping never goes negative.
    MOD:  { precedence: 5, apply: (a, b) => Number(a) - Number(b) * Math.floor(divide(a, b)) },
  };

  const functions = {
    INT: (value) => Math.floor(Number(value)),
    ABS: (value) => Math.abs(Number(value)),
    SQR: (value) => Math.sqrt(Number(value)),
    SIN: (value) => Math.sin(Number(value)),
    COS: (value) => Math.cos(Number(value)),
    TAN: (value) => Math.tan(Number(value)),
    EXP: (value) => Math.exp(Number(value)),
    LOG: (value) => Math.log(Number(value)),
    SGN: (value) => Math.sign(Number(value)),
    FIX: (value) => Math.trunc(Number(value)),
    RND: (maximum = 1) => Math.random() * Number(maximum),
    LEN: (value) => String(value).length,
    'STR$': (value) => String(value),
    'CHR$': (value) => String.fromCharCode(Number(value)),
    'MID$': (text, start = 1, length = Infinity) => [...String(text)].slice(Math.max(1, Number(start)) - 1).slice(0, Math.max(0, Number(length))).join(''),
    'LEFT$': (text, length = 0) => [...String(text)].slice(0, Math.max(0, Number(length))).join(''),
    'RIGHT$': (text, length = 0) => { const chars = [...String(text)]; return chars.slice(chars.length - Math.min(chars.length, Math.max(0, Number(length)))).join(''); },
    VAL: (text) => { const match = /^\s*[-+]?(\d+\.?\d*|\.\d+)/.exec(String(text)); return match ? Number(match[0]) : 0; },
    ASC: (text) => (String(text).codePointAt(0) ?? 0),
    COLOR(...args) {
      if (args.length < 3 || args.length > 4) throw Error('COLOR needs red,green,blue[,alpha]');
      const [r, g, b, a = 1] = args.map(Number);
      return { blipColor: true, r, g, b, a };
    },
  };

  function tokenize(source) {
    const pattern = /\s*(?:([0-9]+(?:\.[0-9]*)?|\.[0-9]+)|("(?:[^"]|"")*")|([A-Za-z_][A-Za-z0-9_$]*)|(<=|>=|<>|[+\-*/(),=<>]))/gy;
    const tokens = [];
    let at = 0;
    while (at < source.length) {
      pattern.lastIndex = at;
      const match = pattern.exec(source);
      if (!match) {
        if (/^\s*$/.test(source.slice(at))) break;
        throw Error('Syntax near ' + source.slice(at, at + 12));
      }
      if (match[1] !== undefined) tokens.push({ kind: 'number', value: Number(match[1]) });
      else if (match[2] !== undefined) tokens.push({ kind: 'string', value: match[2].slice(1, -1).replace(/""/g, '"') });
      else tokens.push({ kind: 'symbol', value: (match[3] || match[4]).toUpperCase() });
      at = pattern.lastIndex;
    }
    return tokens;
  }

  class Expression {
    constructor(source, host) {
      this.tokens = tokenize(source);
      this.at = 0;
      this.host = host;
    }

    peek() { return this.tokens[this.at]?.value; }

    take(value) {
      if (this.peek() !== value) return false;
      this.at++;
      return true;
    }

    need(value) {
      if (!this.take(value)) throw Error('Expected ' + value);
    }

    expression(minimum = 0) {
      let left = this.atom();
      while (Object.hasOwn(operators, this.peek())) {
        const operator = operators[this.peek()];
        if (operator.precedence < minimum) break;
        this.at++;
        left = operator.apply(left, this.expression(operator.precedence + 1));
      }
      return left;
    }

    atom() {
      const token = this.tokens[this.at++];
      if (!token) throw Error('Expression expected');
      if (token.kind !== 'symbol') return token.value;
      switch (token.value) {
        case '(': {
          const value = this.expression();
          this.need(')');
          return value;
        }
        case '-': return -Number(this.atom());
        case '+': return Number(this.atom());
        case 'NOT': return Number(!Number(this.atom()));
      }
      const name = token.value;
      if (!/^[A-Z_]/.test(name)) throw Error('Expression expected');
      if (!this.take('(')) return this.host.read(name);
      const args = this.arguments();
      return Object.hasOwn(functions, name) ? functions[name](...args) : this.host.call(name, args);
    }

    arguments() {
      const args = [];
      if (!this.take(')')) {
        do { args.push(this.expression()); } while (this.take(','));
        this.need(')');
      }
      return args;
    }
  }

  function evaluate(source, host) {
    const parser = new Expression(source, host);
    const value = parser.expression();
    if (parser.at !== parser.tokens.length) throw Error('Syntax near ' + parser.peek());
    return value;
  }

  root.BlipBasicLanguage = Object.freeze({ evaluate });
})(globalThis);
