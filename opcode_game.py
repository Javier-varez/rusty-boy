#!/usr/bin/env python3

import json
from enum import IntEnum
from functools import reduce
from argparse import ArgumentParser
from prettytable import PrettyTable
from colorama import Fore

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

class Register(IntEnum):
    A = 7
    B = 0
    C = 1
    D = 2
    E = 3
    H = 4
    L = 5

class RegisterPair(IntEnum):
    BC = 0
    DE = 1
    HL = 2
    SP = 3

class RegisterPairStack(IntEnum):
    BC = 0
    DE = 1
    HL = 2
    AF = 3

class RegisterPairMem(IntEnum):
    BC = 0
    DE = 1
    HLINC = 2
    HLDEC = 3

class Condition(IntEnum):
    NZ = 0
    Z = 1
    NC = 2
    C = 3

class ResetTarget(IntEnum):
    Target0x00 = 0
    Target0x08 = 1
    Target0x10 = 2
    Target0x18 = 3
    Target0x20 = 4
    Target0x28 = 5
    Target0x30 = 6
    Target0x38 = 7

patterns = [
    (Pattern('01rrrRRR'), Binding('r', Register), Binding('R', Register)),
    (Pattern('01rrr110'), Binding('r', Register)),
    (Pattern('01110RRR'), Binding('R', Register)),
    (Pattern('01110110'),),
    (Pattern('00rrr110'), Binding('r', Register)),
    (Pattern('00110110'),),
    (Pattern('10000rrr'), Binding('r', Register)),
    (Pattern('10010rrr'), Binding('r', Register)),
    (Pattern('10100rrr'), Binding('r', Register)),
    (Pattern('10110rrr'), Binding('r', Register)),
    (Pattern('10001rrr'), Binding('r', Register)),
    (Pattern('10011rrr'), Binding('r', Register)),
    (Pattern('10101rrr'), Binding('r', Register)),
    (Pattern('10111rrr'), Binding('r', Register)),
    (Pattern('00RRR100'), Binding('R', Register)),
    (Pattern('00RRR101'), Binding('R', Register)),
    (Pattern('11001011'),),
    (Pattern('00000000'),),
    (Pattern('00RR0001'), Binding('R', RegisterPair)),
    (Pattern('00RR0010'), Binding('R', RegisterPairMem)),
    (Pattern('00RR1010'), Binding('R', RegisterPairMem)),
    (Pattern('00001000'),),
    (Pattern('00RR0011'), Binding('R', RegisterPair)),
    (Pattern('00RR1011'), Binding('R', RegisterPair)),
    (Pattern('00rr1001'), Binding('r', RegisterPair)),
    (Pattern('00000111'),),
    (Pattern('00001111'),),
    (Pattern('00010111'),),
    (Pattern('00011111'),),
    (Pattern('00100111'),),
    (Pattern('00101111'),),
    (Pattern('00110111'),),
    (Pattern('00111111'),),
    (Pattern('00011000'),),
    (Pattern('001cc000'), Binding('c', Condition)),
    (Pattern('00010000'),),
    (Pattern('11000110'),),
    (Pattern('11001110'),),
    (Pattern('11010110'),),
    (Pattern('11011110'),),
    (Pattern('11100110'),),
    (Pattern('11101110'),),
    (Pattern('11110110'),),
    (Pattern('11111110'),),
    (Pattern('110cc000'), Binding('c', Condition)),
    (Pattern('11001001'),),
    (Pattern('11011001'),),
    (Pattern('110cc010'), Binding('c', Condition)),
    (Pattern('11000011'),),
    (Pattern('11101001'),),
    (Pattern('110cc100'), Binding('c', Condition)),
    (Pattern('11001101'),),
    (Pattern('11ttt111'), Binding('t', ResetTarget)),
    (Pattern('11rr0001'), Binding('r', RegisterPairStack)),
    (Pattern('11rr0101'), Binding('r', RegisterPairStack)),
    (Pattern('11100010'),),
    (Pattern('11110010'),),
    (Pattern('11100000'),),
    (Pattern('11110000'),),
    (Pattern('11101010'),),
    (Pattern('11111010'),),
    (Pattern('11101000'),),
    (Pattern('11111000'),),
    (Pattern('11111001'),),
    (Pattern('11110011'),),
    (Pattern('11111011'),),
    (Pattern('00110100'),),
    (Pattern('00110101'),),
    (Pattern('10000110'),),
    (Pattern('10001110'),),
    (Pattern('10010110'),),
    (Pattern('10011110'),),
    (Pattern('10100110'),),
    (Pattern('10101110'),),
    (Pattern('10110110'),),
    (Pattern('10111110'),),
    (Pattern('11010011'),),
    (Pattern('11100011'),),
    (Pattern('11100100'),),
    (Pattern('11110100'),),
    (Pattern('11011011'),),
    (Pattern('11101011'),),
    (Pattern('11101100'),),
    (Pattern('11111100'),),
    (Pattern('11011101'),),
    (Pattern('11101101'),),
    (Pattern('11111101'),),
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

def opcode_table(args):
    opcodes = read_opcodes()

    resolved = []
    for pattern, *bindings in patterns:
        resolved.extend(pattern.resolve_all(bindings))

    # mark as implemented
    for p in resolved:
        if opcodes[p.as_opcode()].is_implemented():
            raise Exception(f'conflicting generation of opcode: {p.as_opcode()}')
        opcodes[p.as_opcode()].implemented = True

    table = PrettyTable()
    table.header = False

    table.add_row([f"x{i-1:x}" if i > 0 else ' ' for i in range(17)])
    for i in range(16):
        row = [f"{i:x}x"]
        for j in range(16):
            op = opcodes[i * 16 + j]
            if op.is_implemented():
                row.append(f"{Fore.GREEN}{op}{Fore.WHITE}")
            else:
                row.append(f"{Fore.RED}{op}{Fore.WHITE}")
        table.add_row(row)
    print(table)


def default_handler(args):
    print('No command selected')

if __name__ == '__main__':
    parser = ArgumentParser(prog='opcode_game.py', description='gotta catch them all!')
    parser.set_defaults(handler=default_handler)
    subparsers = parser.add_subparsers(title='subcommands')

    ls_parser = subparsers.add_parser('ls')
    ls_parser.add_argument('-u', action='store_true')
    ls_parser.set_defaults(handler=ls_opcodes)

    table_parser = subparsers.add_parser('table')
    table_parser.set_defaults(handler=opcode_table)

    args = parser.parse_args()
    args.handler(args)
