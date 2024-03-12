#!/usr/bin/env python3

import json
from enum import IntEnum
from functools import reduce
from argparse import ArgumentParser

class Register(IntEnum):
    A = 7
    B = 0
    C = 1
    D = 2
    E = 3
    H = 4
    L = 5

class Opcode:
    def __init__(self, mnemonic, operands):
        self.mnemonic = mnemonic
        self.operands = operands
        self.implemented = False

    def __repr__(self):
        res = self.mnemonic
        for operand in self.operands:
            if operand['immediate']:
                res += ' ' + operand['name']
            else:
                res += ' [' + operand['name'] + ']'
        return res

    def is_implemented(self):
        return self.implemented

def read_opcodes():
    all_opcodes = {}
    with open('opcodes.json') as f:
        opcodes = json.loads(f.read())
        unprefixed = opcodes["unprefixed"]
        for k, v in unprefixed.items():
            index = int(k, 0)
            all_opcodes[index] = Opcode(v['mnemonic'], v['operands'])
    return all_opcodes


class Binding:
    def __init__(self, name, ty):
        self.name = name
        self.ty = ty

    def __repr__(self):
        return f'Binding({self.name}, {self.ty})'

class Pattern:
    def __init__(self, pattern):
        self.pattern = pattern

    def resolve(self, binding, value):
        if not isinstance(value, binding.ty):
            raise Exception('Not a binding!')

        l = self.pattern.index(binding.name)
        r = self.pattern.rindex(binding.name)
        match = self.pattern[l:r+1]
        contiguous = reduce(lambda res, x: x == binding.name and res, match, True)
        if not contiguous:
            raise Exception(f'The pattern {self.pattern} is not contiguous')

        num_bits = r - l + 1
        coded = int(value) & ((1 << num_bits) - 1)

        return Pattern(self.pattern[:l] + "{0:0{width}b}".format(coded, width = num_bits) + self.pattern[r+1:])

    def resolve_all(self, bindings):
        resolved = [self]
        for binding in bindings:
            result = []
            for p in resolved:
                for value in binding.ty:
                    result.append(p.resolve(binding, value))
            resolved = result
        return resolved

    def as_opcode(self):
        return int(self.pattern, 2)

    def __repr__(self):
        return f'Pattern({self.pattern})'

patterns = [
    (Pattern('01rrrRRR'), Binding('r', Register), Binding('R', Register)),
    (Pattern('01rrr110'), Binding('r', Register)),
    (Pattern('01110RRR'), Binding('R', Register)),
    (Pattern('01110110'),),
    (Pattern('00rrr110'), Binding('r', Register)),
    (Pattern('00110110'),),
]

def ls_opcodes(args):
    opcodes = read_opcodes()

    resolved = []
    for pattern, *bindings in patterns:
        resolved.extend(pattern.resolve_all(bindings))

    # mark as implemented
    for p in resolved:
        opcodes[p.as_opcode()].implemented = True

    for i, opcode in opcodes.items():
        if args.u:
            if not opcode.implemented:
                print(f"{i:#x} = {opcode}")
        else:
            print(f"{i:#x} = {opcode}, implemented = {opcode.is_implemented()}")

def default_handler(args):
    print('No command selected')

if __name__ == '__main__':
    parser = ArgumentParser(prog='opcode_game.py', description='gotta catch them all!')
    parser.set_defaults(handler=default_handler)
    subparsers = parser.add_subparsers(title='subcommands')

    ls_parser = subparsers.add_parser('ls')
    ls_parser.add_argument('-u', action='store_true')
    ls_parser.set_defaults(handler=ls_opcodes)

    args = parser.parse_args()
    args.handler(args)
