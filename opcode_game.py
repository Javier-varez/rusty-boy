#!/usr/bin/env python3

import os
import json
import toml
from argparse import ArgumentParser

class Opcode:
    def __init__(self, mnemonic, operands, bytes, cycles):
        self.mnemonic = mnemonic
        self.operands = operands
        self.bytes = bytes
        self.cycles = cycles

    def __repr__(self):
        res = self.mnemonic
        for operand in self.operands:
            if operand['immediate']:
                res += ' ' + operand['name']
            elif "increment" in operand and operand["increment"]:
                res += ' [' + operand['name'] + '+]'
            elif "decrement" in operand and operand["decrement"]:
                res += ' [' + operand['name'] + '-]'
            else:
                res += ' [' + operand['name'] + ']'
        return res

def read_opcodes():
    all_opcodes = {}
    with open('opcodes.json') as f:
        opcodes = json.loads(f.read())
        unprefixed = opcodes["unprefixed"]
        for k, v in unprefixed.items():
            index = int(k, 0)
            all_opcodes[index] = Opcode(v['mnemonic'], v['operands'], v['bytes'], v['cycles'])
    return all_opcodes

def read_prefixed_opcodes():
    all_opcodes = {}
    with open('opcodes.json') as f:
        opcodes = json.loads(f.read())
        cbprefixed = opcodes["cbprefixed"]
        for k, v in cbprefixed.items():
            index = int(k, 0)
            all_opcodes[index] = Opcode(v['mnemonic'], v['operands'], v['bytes'], v['cycles'])
    return all_opcodes

def testgen_ld_reg_reg(id, opcode, outpath):
    assert len(opcode.operands) == 2
    assert len(opcode.cycles) == 1

    dest_op = opcode.operands[0]["name"].lower()
    src_op = opcode.operands[1]["name"].lower()

    tests = {
        "move-nonzero" : {
            "cycles" : opcode.cycles[0],
            "entry_state" : {
                dest_op: 0,
                src_op: 0x82,
            },
            "exit_state" : {
                dest_op: 0x82,
                src_op: 0x82,
                'pc': opcode.bytes
            },
            "program" : { "instructions" : [id] }
        },
        "move-zero" : {
            "cycles" : opcode.cycles[0],
            "entry_state" : {
                dest_op: 0x82,
                src_op: 0x00,
            },
            "exit_state" : {
                dest_op: 0x00,
                src_op: 0x00,
                'pc': opcode.bytes
            },
            "program" : { "instructions" : [id] }
        }
    }

    with open(os.path.join(outpath, f"ld_{dest_op}_{src_op}.toml"), 'w') as f:
        toml.dump(tests, f)

def testgen_ld_ind_reg_reg(id, opcode, outpath):
    assert len(opcode.operands) == 2
    assert len(opcode.cycles) == 1

    src_op = opcode.operands[1]["name"].lower()

    tests = {
        "move-nonzero" : {
            "cycles" : opcode.cycles[0],
            "entry_state" : {
                'h': 0x12,
                'l': 0x34,
                src_op: 0x82,
            },
            "exit_state" : {
                'h': 0x12,
                'l': 0x34,
                src_op: 0x82,
                'pc': opcode.bytes,
                'memory': { '0x1234' : [0x82] }
            },
            "program" : { "instructions" : [id] }
        },
        "move-zero" : {
            "cycles" : opcode.cycles[0],
            "entry_state" : {
                'h': 0x12,
                'l': 0x34,
                src_op: 0x00,
                'memory': { '0x1234' : [0x82] }
            },
            "exit_state" : {
                'h': 0x12,
                'l': 0x34,
                src_op: 0x00,
                'pc': opcode.bytes,
            },
            "program" : { "instructions" : [id] }
        }
    }

    with open(os.path.join(outpath, f"ld_ind_hl_{src_op}.toml"), 'w') as f:
        toml.dump(tests, f)

def testgen_ld_reg_ind_reg(id, opcode, outpath):
    assert len(opcode.operands) == 2
    assert len(opcode.cycles) == 1

    dest_op = opcode.operands[0]["name"].lower()

    tests = {
        "move-nonzero" : {
            "cycles" : opcode.cycles[0],
            "entry_state" : {
                'h': 0x12,
                'l': 0x34,
                dest_op: 0x43,
                'memory': { '0x1234' : [0x82] }
            },
            "exit_state" : {
                'h': 0x12,
                'l': 0x34,
                dest_op: 0x82,
                'pc': opcode.bytes,
                'memory': { '0x1234' : [0x82] }
            },
            "program" : { "instructions" : [id] }
        },
        "move-zero" : {
            "cycles" : opcode.cycles[0],
            "entry_state" : {
                'h': 0x12,
                'l': 0x34,
                dest_op: 0x43,
            },
            "exit_state" : {
                'h': 0x12,
                'l': 0x34,
                dest_op: 0x0,
                'pc': opcode.bytes,
            },
            "program" : { "instructions" : [id] }
        }
    }

    with open(os.path.join(outpath, f"ld_{dest_op}_ind_hl.toml"), 'w') as f:
        toml.dump(tests, f)

def testgen_add_reg_reg(id, opcode, outpath):
    assert len(opcode.operands) == 2
    assert len(opcode.cycles) == 1

    if not opcode.operands[1]['immediate']:
        return

    dest_op = opcode.operands[0]["name"].lower()
    src_op = opcode.operands[1]["name"].lower()

    if dest_op == src_op:
        with open(os.path.join(outpath, f"add_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[no_carry]
cycles = {opcode.cycles[0]}

[no_carry.entry_state]
{dest_op} = 0x12
flags = ['Z', 'N', 'C', 'H']

[no_carry.exit_state]
{dest_op} = 0x24
flags = []
pc = {opcode.bytes}

[no_carry.program]
instructions = [
    {id:#x}, # add {dest_op}, {src_op}
]


[half_carry]
cycles = {opcode.cycles[0]}

[half_carry.entry_state]
{dest_op} = 0x18
flags = ['Z', 'N', 'C', 'H']

[half_carry.exit_state]
{dest_op} = 0x30
flags = ['H']
pc = {opcode.bytes}

[half_carry.program]
instructions = [
{id:#x}, # add {dest_op}, {src_op}
]

[carry]
cycles = {opcode.cycles[0]}

[carry.entry_state]
{dest_op} = 0x82
flags = ['Z', 'N', 'C', 'H']

[carry.exit_state]
{dest_op} = 0x04
flags = ['C']
pc = {opcode.bytes}

[carry.program]
instructions = [
{id:#x}, # add {dest_op}, {src_op}
]

[zero]
cycles = {opcode.cycles[0]}

[zero.entry_state]
{dest_op} = 0x80
flags = ['Z', 'N', 'C', 'H']

[zero.exit_state]
{dest_op} = 0x00
flags = ['C', 'Z']
pc = {opcode.bytes}

[zero.program]
instructions = [
{id:#x}, # add {dest_op}, {src_op}
]
''')
    else:
        with open(os.path.join(outpath, f"add_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[no_carry]
cycles = {opcode.cycles[0]}

[no_carry.entry_state]
{dest_op} = 0x12
{src_op} = 0xE4
flags = ['Z', 'N', 'C', 'H']

[no_carry.exit_state]
{dest_op} = 0xF6
{src_op} = 0xE4
flags = []
pc = {opcode.bytes}

[no_carry.program]
instructions = [
    {id:#x}, # add {dest_op}, {src_op}
]


[half_carry]
cycles = {opcode.cycles[0]}

[half_carry.entry_state]
{dest_op} = 0x12
{src_op} = 0xDF
flags = ['Z', 'N', 'C', 'H']

[half_carry.exit_state]
{dest_op} = 0xF1
{src_op} = 0xDF
flags = ['H']
pc = {opcode.bytes}

[half_carry.program]
instructions = [
{id:#x}, # add {dest_op}, {src_op}
]

[carry]
cycles = {opcode.cycles[0]}

[carry.entry_state]
{dest_op} = 0x12
{src_op} = 0xF4
flags = ['Z', 'N', 'C', 'H']

[carry.exit_state]
{dest_op} = 0x06
{src_op} = 0xF4
flags = ['C']
pc = {opcode.bytes}

[carry.program]
instructions = [
{id:#x}, # add {dest_op}, {src_op}
]

[zero]
cycles = {opcode.cycles[0]}

[zero.entry_state]
{dest_op} = 0x01
{src_op} = 0xFF
flags = ['Z', 'N', 'C', 'H']

[zero.exit_state]
{dest_op} = 0x00
{src_op} = 0xFF
flags = ['C', 'H', 'Z']
pc = {opcode.bytes}

[zero.program]
instructions = [
{id:#x}, # add {dest_op}, {src_op}
]
''')

def testgen_adc_reg_reg(id, opcode, outpath):
    assert len(opcode.operands) == 2
    assert len(opcode.cycles) == 1

    if not opcode.operands[1]['immediate']:
        return

    dest_op = opcode.operands[0]["name"].lower()
    src_op = opcode.operands[1]["name"].lower()

    if dest_op == src_op:
        with open(os.path.join(outpath, f"adc_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[no_carry]
cycles = {opcode.cycles[0]}

[no_carry.entry_state]
{dest_op} = 0x12
flags = ['Z', 'N', 'C', 'H']

[no_carry.exit_state]
{dest_op} = 0x25
flags = []
pc = {opcode.bytes}

[no_carry.program]
instructions = [
    {id:#x}, # adc {dest_op}, {src_op}
]


[half_carry]
cycles = {opcode.cycles[0]}

[half_carry.entry_state]
{dest_op} = 0x18
flags = ['Z', 'N', 'C', 'H']

[half_carry.exit_state]
{dest_op} = 0x31
flags = ['H']
pc = {opcode.bytes}

[half_carry.program]
instructions = [
    {id:#x}, # adc {dest_op}, {src_op}
]

[carry]
cycles = {opcode.cycles[0]}

[carry.entry_state]
{dest_op} = 0x82
flags = ['Z', 'N', 'C', 'H']

[carry.exit_state]
{dest_op} = 0x05
flags = ['C']
pc = {opcode.bytes}

[carry.program]
instructions = [
    {id:#x}, # adc {dest_op}, {src_op}
]

[zero]
cycles = {opcode.cycles[0]}

[zero.entry_state]
{dest_op} = 0x80
flags = ['Z', 'N', 'H']

[zero.exit_state]
{dest_op} = 0x00
flags = ['C', 'Z']
pc = {opcode.bytes}

[zero.program]
instructions = [
    {id:#x}, # adc {dest_op}, {src_op}
]
''')
    else:
        with open(os.path.join(outpath, f"adc_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[no_carry]
cycles = {opcode.cycles[0]}

[no_carry.entry_state]
{dest_op} = 0x12
{src_op} = 0xE4
flags = ['Z', 'N', 'C', 'H']

[no_carry.exit_state]
{dest_op} = 0xF7
{src_op} = 0xE4
flags = []
pc = {opcode.bytes}

[no_carry.program]
instructions = [
    {id:#x}, # adc {dest_op}, {src_op}
]


[half_carry]
cycles = {opcode.cycles[0]}

[half_carry.entry_state]
{dest_op} = 0x12
{src_op} = 0xDF
flags = ['Z', 'N', 'C', 'H']

[half_carry.exit_state]
{dest_op} = 0xF2
{src_op} = 0xDF
flags = ['H']
pc = {opcode.bytes}

[half_carry.program]
instructions = [
    {id:#x}, # adc {dest_op}, {src_op}
]

[carry]
cycles = {opcode.cycles[0]}

[carry.entry_state]
{dest_op} = 0x12
{src_op} = 0xF4
flags = ['Z', 'N', 'C', 'H']

[carry.exit_state]
{dest_op} = 0x07
{src_op} = 0xF4
flags = ['C']
pc = {opcode.bytes}

[carry.program]
instructions = [
    {id:#x}, # adc {dest_op}, {src_op}
]

[zero]
cycles = {opcode.cycles[0]}

[zero.entry_state]
{dest_op} = 0x01
{src_op} = 0xFE
flags = ['Z', 'N', 'C', 'H']

[zero.exit_state]
{dest_op} = 0x00
{src_op} = 0xFE
flags = ['C', 'H', 'Z']
pc = {opcode.bytes}

[zero.program]
instructions = [
    {id:#x}, # adc {dest_op}, {src_op}
]
''')


def testgen_sub_reg_reg(id, opcode, outpath):
    assert len(opcode.operands) == 2
    assert len(opcode.cycles) == 1

    if not opcode.operands[1]['immediate']:
        return

    dest_op = opcode.operands[0]["name"].lower()
    src_op = opcode.operands[1]["name"].lower()

    if dest_op == src_op:
        with open(os.path.join(outpath, f"sub_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
{dest_op} = 0x12
flags = ['C', 'H']

[test.exit_state]
{dest_op} = 0x00
flags = ['N', 'Z']
pc = {opcode.bytes}

[test.program]
instructions = [
    {id:#x}, # sub {dest_op}, {src_op}
]
''')
    else:
        with open(os.path.join(outpath, f"sub_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[no_carry]
cycles = {opcode.cycles[0]}

[no_carry.entry_state]
{dest_op} = 0xFE
{src_op} = 0x11
flags = ['Z', 'C', 'H']

[no_carry.exit_state]
{dest_op} = 0xED
{src_op} = 0x11
flags = ['N']
pc = {opcode.bytes}

[no_carry.program]
instructions = [
    {id:#x}, # sub {dest_op}, {src_op}
]

[half_carry]
cycles = {opcode.cycles[0]}

[half_carry.entry_state]
{dest_op} = 0xF2
{src_op} = 0x13
flags = []

[half_carry.exit_state]
{dest_op} = 0xDF
{src_op} = 0x13
flags = ['N', 'H']
pc = {opcode.bytes}

[half_carry.program]
instructions = [
    {id:#x}, # sub {dest_op}, {src_op}
]

[carry]
cycles = {opcode.cycles[0]}

[carry.entry_state]
{dest_op} = 0x00
{src_op} = 0x01
flags = []

[carry.exit_state]
{dest_op} = 0xFF
{src_op} = 0x01
flags = ['N', 'H', 'C']
pc = {opcode.bytes}

[carry.program]
instructions = [
    {id:#x}, # sub {dest_op}, {src_op}
]

[zero]
cycles = {opcode.cycles[0]}

[zero.entry_state]
{dest_op} = 0x01
{src_op} = 0x01
flags = []

[zero.exit_state]
{dest_op} = 0x00
{src_op} = 0x01
flags = ['N', 'Z']
pc = {opcode.bytes}

[zero.program]
instructions = [
    {id:#x}, # sub {dest_op}, {src_op}
]
''')

def testgen_sbc_reg_reg(id, opcode, outpath):
    assert len(opcode.operands) == 2
    assert len(opcode.cycles) == 1

    if not opcode.operands[1]['immediate']:
        return

    dest_op = opcode.operands[0]["name"].lower()
    src_op = opcode.operands[1]["name"].lower()

    if dest_op == src_op:
        with open(os.path.join(outpath, f"sbc_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
{dest_op} = 0x12
flags = ['C']

[test.exit_state]
{dest_op} = 0xFF
flags = ['N', 'C', 'H']
pc = {opcode.bytes}

[test.program]
instructions = [
    {id:#x}, # sbc {dest_op}, {src_op}
]
''')
    else:
        with open(os.path.join(outpath, f"sbc_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[no_carry]
cycles = {opcode.cycles[0]}

[no_carry.entry_state]
{dest_op} = 0xFE
{src_op} = 0x11
flags = ['Z', 'C', 'H']

[no_carry.exit_state]
{dest_op} = 0xEC
{src_op} = 0x11
flags = ['N']
pc = {opcode.bytes}

[no_carry.program]
instructions = [
    {id:#x}, # sbc {dest_op}, {src_op}
]

[half_carry]
cycles = {opcode.cycles[0]}

[half_carry.entry_state]
{dest_op} = 0xF2
{src_op} = 0x13
flags = ['C']

[half_carry.exit_state]
{dest_op} = 0xDE
{src_op} = 0x13
flags = ['N', 'H']
pc = {opcode.bytes}

[half_carry.program]
instructions = [
    {id:#x}, # sbc {dest_op}, {src_op}
]

[carry]
cycles = {opcode.cycles[0]}

[carry.entry_state]
{dest_op} = 0x00
{src_op} = 0x01
flags = ['C']

[carry.exit_state]
{dest_op} = 0xFE
{src_op} = 0x01
flags = ['N', 'H', 'C']
pc = {opcode.bytes}

[carry.program]
instructions = [
    {id:#x}, # sbc {dest_op}, {src_op}
]

[zero]
cycles = {opcode.cycles[0]}

[zero.entry_state]
{dest_op} = 0x02
{src_op} = 0x01
flags = ['C']

[zero.exit_state]
{dest_op} = 0x00
{src_op} = 0x01
flags = ['N', 'Z']
pc = {opcode.bytes}

[zero.program]
instructions = [
    {id:#x}, # sbc {dest_op}, {src_op}
]
''')

def testgen_and_reg_reg(id, opcode, outpath):
    assert len(opcode.operands) == 2
    assert len(opcode.cycles) == 1

    if not opcode.operands[1]['immediate']:
        return

    dest_op = opcode.operands[0]["name"].lower()
    src_op = opcode.operands[1]["name"].lower()

    if dest_op == src_op:
        with open(os.path.join(outpath, f"and_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[non_zero]
cycles = {opcode.cycles[0]}

[non_zero.entry_state]
{dest_op} = 0x12
flags = ['C', 'N', 'Z']

[non_zero.exit_state]
{dest_op} = 0x12
flags = ['H']
pc = {opcode.bytes}

[non_zero.program]
instructions = [
    {id:#x}, # and {dest_op}, {src_op}
]
''')
    else:
        with open(os.path.join(outpath, f"and_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[non_zero]
cycles = {opcode.cycles[0]}

[non_zero.entry_state]
{dest_op} = 0xF0
{src_op} = 0x5F
flags = ['Z', 'C', 'H']

[non_zero.exit_state]
{dest_op} = 0x50
{src_op} = 0x5F
flags = ['H']
pc = {opcode.bytes}

[non_zero.program]
instructions = [
    {id:#x}, # and {dest_op}, {src_op}
]

[zero]
cycles = {opcode.cycles[0]}

[zero.entry_state]
{dest_op} = 0xA5
{src_op} = 0x5A
flags = ['C']

[zero.exit_state]
{dest_op} = 0x00
{src_op} = 0x5A
flags = ['Z', 'H']
pc = {opcode.bytes}

[zero.program]
instructions = [
    {id:#x}, # and {dest_op}, {src_op}
]
''')

def testgen_or_reg_reg(id, opcode, outpath):
    assert len(opcode.operands) == 2
    assert len(opcode.cycles) == 1

    if not opcode.operands[1]['immediate']:
        return

    dest_op = opcode.operands[0]["name"].lower()
    src_op = opcode.operands[1]["name"].lower()

    if dest_op == src_op:
        with open(os.path.join(outpath, f"or_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[non_zero]
cycles = {opcode.cycles[0]}

[non_zero.entry_state]
{dest_op} = 0x12
flags = ['C', 'N', 'Z']

[non_zero.exit_state]
{dest_op} = 0x12
flags = []
pc = {opcode.bytes}

[non_zero.program]
instructions = [
    {id:#x}, # or {dest_op}, {src_op}
]
''')
    else:
        with open(os.path.join(outpath, f"or_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[non_zero]
cycles = {opcode.cycles[0]}

[non_zero.entry_state]
{dest_op} = 0x60
{src_op} = 0x5F
flags = ['Z', 'C', 'H']

[non_zero.exit_state]
{dest_op} = 0x7F
{src_op} = 0x5F
flags = []
pc = {opcode.bytes}

[non_zero.program]
instructions = [
    {id:#x}, # or {dest_op}, {src_op}
]

[zero]
cycles = {opcode.cycles[0]}

[zero.entry_state]
{dest_op} = 0x00
{src_op} = 0x00
flags = ['C']

[zero.exit_state]
{dest_op} = 0x00
{src_op} = 0x00
flags = ['Z']
pc = {opcode.bytes}

[zero.program]
instructions = [
    {id:#x}, # or {dest_op}, {src_op}
]
''')

def testgen_xor_reg_reg(id, opcode, outpath):
    assert len(opcode.operands) == 2
    assert len(opcode.cycles) == 1

    if not opcode.operands[1]['immediate']:
        return

    dest_op = opcode.operands[0]["name"].lower()
    src_op = opcode.operands[1]["name"].lower()

    if dest_op == src_op:
        with open(os.path.join(outpath, f"xor_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[non_zero]
cycles = {opcode.cycles[0]}

[non_zero.entry_state]
{dest_op} = 0x12
flags = ['C', 'N', 'Z']

[non_zero.exit_state]
{dest_op} = 0x00
flags = ['Z']
pc = {opcode.bytes}

[non_zero.program]
instructions = [
    {id:#x}, # xor {dest_op}, {src_op}
]
''')
    else:
        with open(os.path.join(outpath, f"xor_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[non_zero]
cycles = {opcode.cycles[0]}

[non_zero.entry_state]
{dest_op} = 0x60
{src_op} = 0x5F
flags = ['Z', 'C', 'H']

[non_zero.exit_state]
{dest_op} = 0x3F
{src_op} = 0x5F
flags = []
pc = {opcode.bytes}

[non_zero.program]
instructions = [
    {id:#x}, # xor {dest_op}, {src_op}
]

[zero]
cycles = {opcode.cycles[0]}

[zero.entry_state]
{dest_op} = 0x12
{src_op} = 0x12
flags = ['C']

[zero.exit_state]
{dest_op} = 0x00
{src_op} = 0x12
flags = ['Z']
pc = {opcode.bytes}

[zero.program]
instructions = [
    {id:#x}, # xor {dest_op}, {src_op}
]
''')

def testgen_cp_reg_reg(id, opcode, outpath):
    assert len(opcode.operands) == 2
    assert len(opcode.cycles) == 1

    if not opcode.operands[1]['immediate']:
        return

    dest_op = opcode.operands[0]["name"].lower()
    src_op = opcode.operands[1]["name"].lower()

    if dest_op == src_op:
        with open(os.path.join(outpath, f"cp_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
{dest_op} = 0x12
flags = ['C', 'H']

[test.exit_state]
{dest_op} = 0x12
flags = ['N', 'Z']
pc = {opcode.bytes}

[test.program]
instructions = [
    {id:#x}, # cp {dest_op}, {src_op}
]
''')
    else:
        with open(os.path.join(outpath, f"cp_{dest_op}_{src_op}.toml"), 'w') as f:
            f.write(f'''[no_carry]
cycles = {opcode.cycles[0]}

[no_carry.entry_state]
{dest_op} = 0xFE
{src_op} = 0x11
flags = ['Z', 'C', 'H']

[no_carry.exit_state]
{dest_op} = 0xFE
{src_op} = 0x11
flags = ['N']
pc = {opcode.bytes}

[no_carry.program]
instructions = [
    {id:#x}, # cp {dest_op}, {src_op}
]

[half_carry]
cycles = {opcode.cycles[0]}

[half_carry.entry_state]
{dest_op} = 0xF2
{src_op} = 0x13
flags = []

[half_carry.exit_state]
{dest_op} = 0xF2
{src_op} = 0x13
flags = ['N', 'H']
pc = {opcode.bytes}

[half_carry.program]
instructions = [
    {id:#x}, # cp {dest_op}, {src_op}
]

[carry]
cycles = {opcode.cycles[0]}

[carry.entry_state]
{dest_op} = 0x00
{src_op} = 0x01
flags = []

[carry.exit_state]
{dest_op} = 0x00
{src_op} = 0x01
flags = ['N', 'H', 'C']
pc = {opcode.bytes}

[carry.program]
instructions = [
    {id:#x}, # cp {dest_op}, {src_op}
]

[zero]
cycles = {opcode.cycles[0]}

[zero.entry_state]
{dest_op} = 0x01
{src_op} = 0x01
flags = []

[zero.exit_state]
{dest_op} = 0x01
{src_op} = 0x01
flags = ['N', 'Z']
pc = {opcode.bytes}

[zero.program]
instructions = [
    {id:#x}, # cp {dest_op}, {src_op}
]
''')

def testgen_rlc(id, opcode, outpath):
    assert len(opcode.operands) == 1
    assert len(opcode.cycles) == 1

    op = opcode.operands[0]["name"].lower()
    immediate = opcode.operands[0]['immediate']

    if immediate:
        with open(os.path.join(outpath, f"rlc_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
{op} = 0xA5
flags = ['Z', 'C', 'H']

[test.exit_state]
{op} = 0x4B
flags = ['C']
pc = {opcode.bytes}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # rlc {op}
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
{op} = 0xA5
flags = ['Z', 'H']

[test2.exit_state]
{op} = 0x4B
flags = ['C']
pc = {opcode.bytes}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # rlc {op}
]

[test3]
cycles = {opcode.cycles[0]}

[test3.entry_state]
{op} = 0x25
flags = ['Z', 'C', 'H']

[test3.exit_state]
{op} = 0x4A
flags = []
pc = {opcode.bytes}

[test3.program]
instructions = [
    0xCB,
    {id:#x}, # rlc {op}
]

[test4]
cycles = {opcode.cycles[0]}

[test4.entry_state]
{op} = 0x00
flags = ['N', 'C', 'H']

[test4.exit_state]
{op} = 0x00
flags = ['Z']
pc = {opcode.bytes}

[test4.program]
instructions = [
    0xCB,
    {id:#x}, # rlc {op}
]
''')
    else:
        with open(os.path.join(outpath, f"rlc_ind_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'C', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test.exit_state]
h = 0x12
l = 0x34
flags = ['C']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x4B] }}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # rlc [{op}]
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test2.exit_state]
h = 0x12
l = 0x34
flags = ['C']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x4B] }}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # rlc [{op}]
]

[test3]
cycles = {opcode.cycles[0]}

[test3.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'C', 'H']
memory = {{ 0x1234 = [0x25] }}

[test3.exit_state]
h = 0x12
l = 0x34
flags = []
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x4A] }}

[test3.program]
instructions = [
    0xCB,
    {id:#x}, # rlc [{op}]
]

[test4]
cycles = {opcode.cycles[0]}

[test4.entry_state]
h = 0x12
l = 0x34
flags = ['N', 'C', 'H']
memory = {{ 0x1234 = [0x00] }}

[test4.exit_state]
h = 0x12
l = 0x34
flags = ['Z']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x00] }}

[test4.program]
instructions = [
    0xCB,
    {id:#x}, # rlc [{op}]
]
''')

def testgen_rrc(id, opcode, outpath):
    assert len(opcode.operands) == 1
    assert len(opcode.cycles) == 1

    op = opcode.operands[0]["name"].lower()
    immediate = opcode.operands[0]['immediate']

    if immediate:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
{op} = 0xA5
flags = ['Z', 'C', 'H']

[test.exit_state]
{op} = 0xD2
flags = ['C']
pc = {opcode.bytes}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
{op} = 0xA5
flags = ['Z', 'H']

[test2.exit_state]
{op} = 0xD2
flags = ['C']
pc = {opcode.bytes}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test3]
cycles = {opcode.cycles[0]}

[test3.entry_state]
{op} = 0xA4
flags = ['Z', 'C', 'H']

[test3.exit_state]
{op} = 0x52
flags = []
pc = {opcode.bytes}

[test3.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test4]
cycles = {opcode.cycles[0]}

[test4.entry_state]
{op} = 0x00
flags = ['N', 'C', 'H']

[test4.exit_state]
{op} = 0x00
flags = ['Z']
pc = {opcode.bytes}

[test4.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

''')
    else:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_ind_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'C', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test.exit_state]
h = 0x12
l = 0x34
flags = ['C']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0xD2] }}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test2.exit_state]
h = 0x12
l = 0x34
flags = ['C']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0xD2] }}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test3]
cycles = {opcode.cycles[0]}

[test3.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'C', 'H']
memory = {{ 0x1234 = [0xA4] }}

[test3.exit_state]
h = 0x12
l = 0x34
flags = []
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x52] }}

[test3.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test4]
cycles = {opcode.cycles[0]}

[test4.entry_state]
h = 0x12
l = 0x34
flags = ['N', 'C', 'H']
memory = {{ 0x1234 = [0x00] }}

[test4.exit_state]
h = 0x12
l = 0x34
flags = ['Z']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x00] }}

[test4.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

''')

def testgen_rl(id, opcode, outpath):
    assert len(opcode.operands) == 1
    assert len(opcode.cycles) == 1

    op = opcode.operands[0]["name"].lower()
    immediate = opcode.operands[0]['immediate']

    if immediate:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
{op} = 0xA5
flags = ['Z', 'C', 'H']

[test.exit_state]
{op} = 0x4B
flags = ['C']
pc = {opcode.bytes}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
{op} = 0xA5
flags = ['Z', 'H']

[test2.exit_state]
{op} = 0x4A
flags = ['C']
pc = {opcode.bytes}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test3]
cycles = {opcode.cycles[0]}

[test3.entry_state]
{op} = 0x25
flags = ['Z', 'C', 'H']

[test3.exit_state]
{op} = 0x4B
flags = []
pc = {opcode.bytes}

[test3.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test4]
cycles = {opcode.cycles[0]}

[test4.entry_state]
{op} = 0x80
flags = ['N', 'H']

[test4.exit_state]
{op} = 0x00
flags = ['C', 'Z']
pc = {opcode.bytes}

[test4.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]
''')
    else:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_ind_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'C', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test.exit_state]
h = 0x12
l = 0x34
flags = ['C']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x4B] }}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test2.exit_state]
h = 0x12
l = 0x34
flags = ['C']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x4A] }}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test3]
cycles = {opcode.cycles[0]}

[test3.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'C', 'H']
memory = {{ 0x1234 = [0x25] }}

[test3.exit_state]
h = 0x12
l = 0x34
flags = []
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x4B] }}

[test3.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test4]
cycles = {opcode.cycles[0]}

[test4.entry_state]
h = 0x12
l = 0x34
flags = ['N', 'H']
memory = {{ 0x1234 = [0x80] }}

[test4.exit_state]
h = 0x12
l = 0x34
flags = ['C', 'Z']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x00] }}

[test4.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]
''')

def testgen_rr(id, opcode, outpath):
    assert len(opcode.operands) == 1
    assert len(opcode.cycles) == 1

    op = opcode.operands[0]["name"].lower()
    immediate = opcode.operands[0]['immediate']

    if immediate:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
{op} = 0xA5
flags = ['Z', 'C', 'H']

[test.exit_state]
{op} = 0xD2
flags = ['C']
pc = {opcode.bytes}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
{op} = 0xA5
flags = ['Z', 'H']

[test2.exit_state]
{op} = 0x52
flags = ['C']
pc = {opcode.bytes}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test3]
cycles = {opcode.cycles[0]}

[test3.entry_state]
{op} = 0xA4
flags = ['Z', 'C', 'H']

[test3.exit_state]
{op} = 0xD2
flags = []
pc = {opcode.bytes}

[test3.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test4]
cycles = {opcode.cycles[0]}

[test4.entry_state]
{op} = 0x01
flags = ['N', 'H']

[test4.exit_state]
{op} = 0x00
flags = ['C', 'Z']
pc = {opcode.bytes}

[test4.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

''')
    else:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_ind_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'C', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test.exit_state]
h = 0x12
l = 0x34
flags = ['C']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0xD2] }}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test2.exit_state]
h = 0x12
l = 0x34
flags = ['C']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x52] }}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test3]
cycles = {opcode.cycles[0]}

[test3.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'C', 'H']
memory = {{ 0x1234 = [0xA4] }}

[test3.exit_state]
h = 0x12
l = 0x34
flags = []
pc = {opcode.bytes}
memory = {{ 0x1234 = [0xD2] }}

[test3.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test4]
cycles = {opcode.cycles[0]}

[test4.entry_state]
h = 0x12
l = 0x34
flags = ['N', 'H']
memory = {{ 0x1234 = [0x01] }}

[test4.exit_state]
h = 0x12
l = 0x34
flags = ['C', 'Z']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x00] }}

[test4.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

''')

def testgen_sla(id, opcode, outpath):
    assert len(opcode.operands) == 1
    assert len(opcode.cycles) == 1

    op = opcode.operands[0]["name"].lower()
    immediate = opcode.operands[0]['immediate']

    if immediate:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
{op} = 0xA5
flags = ['Z', 'C', 'H']

[test.exit_state]
{op} = 0x4A
flags = ['C']
pc = {opcode.bytes}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
{op} = 0xA5
flags = ['Z', 'H']

[test2.exit_state]
{op} = 0x4A
flags = ['C']
pc = {opcode.bytes}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test3]
cycles = {opcode.cycles[0]}

[test3.entry_state]
{op} = 0x25
flags = ['Z', 'C', 'H']

[test3.exit_state]
{op} = 0x4A
flags = []
pc = {opcode.bytes}

[test3.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]
''')
    else:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_ind_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'C', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test.exit_state]
h = 0x12
l = 0x34
flags = ['C']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x4A] }}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test2.exit_state]
h = 0x12
l = 0x34
flags = ['C']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x4A] }}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test3]
cycles = {opcode.cycles[0]}

[test3.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'C', 'H']
memory = {{ 0x1234 = [0x25] }}

[test3.exit_state]
h = 0x12
l = 0x34
flags = []
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x4A] }}

[test3.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]
''')

def testgen_sra(id, opcode, outpath):
    assert len(opcode.operands) == 1
    assert len(opcode.cycles) == 1

    op = opcode.operands[0]["name"].lower()
    immediate = opcode.operands[0]['immediate']

    if immediate:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
{op} = 0xA5
flags = ['Z', 'C', 'H']

[test.exit_state]
{op} = 0xD2
flags = ['C']
pc = {opcode.bytes}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
{op} = 0xA5
flags = ['Z', 'H']

[test2.exit_state]
{op} = 0xD2
flags = ['C']
pc = {opcode.bytes}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test3]
cycles = {opcode.cycles[0]}

[test3.entry_state]
{op} = 0xA4
flags = ['Z', 'C', 'H']

[test3.exit_state]
{op} = 0xD2
flags = []
pc = {opcode.bytes}

[test3.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

''')
    else:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_ind_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'C', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test.exit_state]
h = 0x12
l = 0x34
flags = ['C']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0xD2] }}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test2.exit_state]
h = 0x12
l = 0x34
flags = ['C']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0xD2] }}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test3]
cycles = {opcode.cycles[0]}

[test3.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'C', 'H']
memory = {{ 0x1234 = [0xA4] }}

[test3.exit_state]
h = 0x12
l = 0x34
flags = []
pc = {opcode.bytes}
memory = {{ 0x1234 = [0xD2] }}

[test3.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

''')

def testgen_srl(id, opcode, outpath):
    assert len(opcode.operands) == 1
    assert len(opcode.cycles) == 1

    op = opcode.operands[0]["name"].lower()
    immediate = opcode.operands[0]['immediate']

    if immediate:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
{op} = 0xA5
flags = ['Z', 'C', 'H']

[test.exit_state]
{op} = 0x52
flags = ['C']
pc = {opcode.bytes}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
{op} = 0xA5
flags = ['Z', 'H']

[test2.exit_state]
{op} = 0x52
flags = ['C']
pc = {opcode.bytes}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test3]
cycles = {opcode.cycles[0]}

[test3.entry_state]
{op} = 0xA4
flags = ['Z', 'C', 'H']

[test3.exit_state]
{op} = 0x52
flags = []
pc = {opcode.bytes}

[test3.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

''')
    else:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_ind_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'C', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test.exit_state]
h = 0x12
l = 0x34
flags = ['C']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x52] }}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test2.exit_state]
h = 0x12
l = 0x34
flags = ['C']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x52] }}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test3]
cycles = {opcode.cycles[0]}

[test3.entry_state]
h = 0x12
l = 0x34
flags = ['Z', 'C', 'H']
memory = {{ 0x1234 = [0xA4] }}

[test3.exit_state]
h = 0x12
l = 0x34
flags = []
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x52] }}

[test3.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

''')

def testgen_swap(id, opcode, outpath):
    assert len(opcode.operands) == 1
    assert len(opcode.cycles) == 1

    op = opcode.operands[0]["name"].lower()
    immediate = opcode.operands[0]['immediate']

    if immediate:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
{op} = 0xA5
flags = ['N', 'C', 'H']

[test.exit_state]
{op} = 0x5A
flags = []
pc = {opcode.bytes}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
{op} = 0x00
flags = ['N', 'C', 'H']

[test2.exit_state]
{op} = 0x00
flags = ['Z']
pc = {opcode.bytes}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} {op}
]
''')
    else:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_ind_{op}.toml"), 'w') as f:
            f.write(f'''
[test]
cycles = {opcode.cycles[0]}

[test.entry_state]
h = 0x12
l = 0x34
flags = ['N', 'C', 'H']
memory = {{ 0x1234 = [0xA5] }}

[test.exit_state]
h = 0x12
l = 0x34
flags = []
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x5A] }}

[test.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]

[test2]
cycles = {opcode.cycles[0]}

[test2.entry_state]
h = 0x12
l = 0x34
flags = ['N', 'C', 'H']
memory = {{ 0x1234 = [0x00] }}

[test2.exit_state]
h = 0x12
l = 0x34
flags = ['Z']
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x00] }}

[test2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()} [{op}]
]
''')

def testgen_bit(id, opcode, outpath):
    assert len(opcode.operands) == 2
    assert len(opcode.cycles) == 1

    bit = int(opcode.operands[0]["name"])
    op = opcode.operands[1]["name"].lower()
    immediate = opcode.operands[1]['immediate']
    mask = 1 << bit
    inverted_mask = (~mask) & 0xFF

    if immediate:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_{bit}_{op}.toml"), 'w') as f:
            f.write(f'''
[bit_set]
cycles = {opcode.cycles[0]}

[bit_set.entry_state]
{op} = {mask:#x}
flags = ['N']

[bit_set.exit_state]
{op} = {mask:#x}
flags = ['H']
pc = {opcode.bytes}

[bit_set.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()}, {bit}, {op}
]

[bit_unset]
cycles = {opcode.cycles[0]}

[bit_unset.entry_state]
{op} = {inverted_mask:#x}
flags = ['N']

[bit_unset.exit_state]
{op} = {inverted_mask:#x}
flags = ['H', 'Z']
pc = {opcode.bytes}

[bit_unset.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()}, {bit}, {op}
]

''')
    else:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_{bit}_ind_{op}.toml"), 'w') as f:
            f.write(f'''
[bit_set]
cycles = {opcode.cycles[0]}

[bit_set.entry_state]
h = 0x12
l = 0x34
flags = ['N']
memory = {{ 0x1234 = [{mask:#x}] }}

[bit_set.exit_state]
h = 0x12
l = 0x34
flags = ['H']
pc = {opcode.bytes}
memory = {{ 0x1234 = [{mask:#x}] }}

[bit_set.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()}, {bit}, [{op}]
]

[bit_unset]
cycles = {opcode.cycles[0]}

[bit_unset.entry_state]
h = 0x12
l = 0x34
flags = ['N']
memory = {{ 0x1234 = [{inverted_mask:#x}] }}

[bit_unset.exit_state]
h = 0x12
l = 0x34
flags = ['H', 'Z']
pc = {opcode.bytes}
memory = {{ 0x1234 = [{inverted_mask:#x}] }}

[bit_unset.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()}, {bit}, [{op}]
]

''')

def testgen_res(id, opcode, outpath):
    assert len(opcode.operands) == 2
    assert len(opcode.cycles) == 1

    bit = int(opcode.operands[0]["name"])
    op = opcode.operands[1]["name"].lower()
    immediate = opcode.operands[1]['immediate']
    mask = 1 << bit
    inverted_mask = (~mask) & 0xFF

    if immediate:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_{bit}_{op}.toml"), 'w') as f:
            f.write(f'''
[reset_bit_1]
cycles = {opcode.cycles[0]}

[reset_bit_1.entry_state]
{op} = {mask:#x}

[reset_bit_1.exit_state]
{op} = 0x00
pc = {opcode.bytes}

[reset_bit_1.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()}, {bit}, {op}
]

[reset_bit_2]
cycles = {opcode.cycles[0]}

[reset_bit_2.entry_state]
{op} = 0xFF
flags = []

[reset_bit_2.exit_state]
{op} = {inverted_mask:#x}
flags = []
pc = {opcode.bytes}

[reset_bit_2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()}, {bit}, {op}
]

''')
    else:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_{bit}_ind_{op}.toml"), 'w') as f:
            f.write(f'''
[reset_bit_1]
cycles = {opcode.cycles[0]}

[reset_bit_1.entry_state]
h = 0x12
l = 0x34
flags = []
memory = {{ 0x1234 = [{mask:#x}] }}

[reset_bit_1.exit_state]
h = 0x12
l = 0x34
flags = []
pc = {opcode.bytes}
memory = {{ 0x1234 = [0x00] }}

[reset_bit_1.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()}, {bit}, [{op}]
]

[reset_bit_2]
cycles = {opcode.cycles[0]}

[reset_bit_2.entry_state]
h = 0x12
l = 0x34
flags = []
memory = {{ 0x1234 = [0xFF] }}

[reset_bit_2.exit_state]
h = 0x12
l = 0x34
flags = []
pc = {opcode.bytes}
memory = {{ 0x1234 = [{inverted_mask:#x}] }}

[reset_bit_2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()}, {bit}, [{op}]
]

''')

def testgen_set(id, opcode, outpath):
    assert len(opcode.operands) == 2
    assert len(opcode.cycles) == 1

    bit = int(opcode.operands[0]["name"])
    op = opcode.operands[1]["name"].lower()
    immediate = opcode.operands[1]['immediate']
    mask = 1 << bit
    inverted_mask = (~mask) & 0xFF

    if immediate:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_{bit}_{op}.toml"), 'w') as f:
            f.write(f'''
[reset_bit_1]
cycles = {opcode.cycles[0]}

[reset_bit_1.entry_state]
{op} = 0x00

[reset_bit_1.exit_state]
{op} = {mask:#x}
pc = {opcode.bytes}

[reset_bit_1.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()}, {bit}, {op}
]

[reset_bit_2]
cycles = {opcode.cycles[0]}

[reset_bit_2.entry_state]
{op} = {inverted_mask:#x}
flags = []

[reset_bit_2.exit_state]
{op} = 0xFF
flags = []
pc = {opcode.bytes}

[reset_bit_2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()}, {bit}, {op}
]

''')
    else:
        with open(os.path.join(outpath, f"{opcode.mnemonic.lower()}_{bit}_ind_{op}.toml"), 'w') as f:
            f.write(f'''
[reset_bit_1]
cycles = {opcode.cycles[0]}

[reset_bit_1.entry_state]
h = 0x12
l = 0x34
flags = []
memory = {{ 0x1234 = [0x00] }}

[reset_bit_1.exit_state]
h = 0x12
l = 0x34
flags = []
pc = {opcode.bytes}
memory = {{ 0x1234 = [{mask:#x}] }}

[reset_bit_1.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()}, {bit}, [{op}]
]

[reset_bit_2]
cycles = {opcode.cycles[0]}

[reset_bit_2.entry_state]
h = 0x12
l = 0x34
flags = []
memory = {{ 0x1234 = [{inverted_mask:#x}] }}

[reset_bit_2.exit_state]
h = 0x12
l = 0x34
flags = []
pc = {opcode.bytes}
memory = {{ 0x1234 = [0xFF] }}

[reset_bit_2.program]
instructions = [
    0xCB,
    {id:#x}, # {opcode.mnemonic.lower()}, {bit}, [{op}]
]

''')

def testgen(args):
    if args.outpath is None:
        print('Please, provide an output path')
        return

    if args.prefixed:
        opcodes = read_prefixed_opcodes()
    else:
        opcodes = read_opcodes()
    r = range(args.start, args.end)

    for opcode_idx, opcode in opcodes.items():
        if not opcode_idx in r:
            continue

        if opcode.mnemonic == 'LD':
            if opcode.operands[0]["immediate"] and opcode.operands[1]["immediate"]:
                testgen_ld_reg_reg(opcode_idx, opcode, args.outpath)
            elif opcode.operands[0]["immediate"] and not opcode.operands[1]["immediate"]:
                testgen_ld_reg_ind_reg(opcode_idx, opcode, args.outpath)
            elif not opcode.operands[0]["immediate"] and opcode.operands[1]["immediate"]:
                testgen_ld_ind_reg_reg(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'ADD':
            testgen_add_reg_reg(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'ADC':
            testgen_adc_reg_reg(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'SUB':
            testgen_sub_reg_reg(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'SBC':
            testgen_sbc_reg_reg(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'AND':
            testgen_and_reg_reg(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'OR':
            testgen_or_reg_reg(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'XOR':
            testgen_xor_reg_reg(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'CP':
            testgen_cp_reg_reg(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'HALT':
            pass
        elif opcode.mnemonic == 'RLC':
            testgen_rlc(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'RRC':
            testgen_rrc(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'RL':
            testgen_rl(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'RR':
            testgen_rr(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'SLA':
            testgen_sla(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'SRA':
            testgen_sra(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'SRL':
            testgen_srl(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'SWAP':
            testgen_swap(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'BIT':
            testgen_bit(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'RES':
            testgen_res(opcode_idx, opcode, args.outpath)
        elif opcode.mnemonic == 'SET':
            testgen_set(opcode_idx, opcode, args.outpath)

if __name__ == '__main__':
    parser = ArgumentParser(prog='opcode_game.py', description='gotta catch them all!')

    def default_handler(args):
        print('No command selected')
        parser.print_help()

    parser.set_defaults(handler=default_handler)
    subparsers = parser.add_subparsers(title='subcommands')

    def any_base_int(x):
        return int(x, 0)

    testgen_parser = subparsers.add_parser('testgen')
    testgen_parser.set_defaults(handler=testgen)
    testgen_parser.add_argument('-s', action='store', default=0, type=any_base_int, dest='start')
    testgen_parser.add_argument('-e', action='store', default=255, type=any_base_int, dest='end')
    testgen_parser.add_argument('-o', action='store', default=None, dest='outpath')
    testgen_parser.add_argument('-c', action='store_true', default=False, dest='prefixed')

    args = parser.parse_args()
    args.handler(args)
