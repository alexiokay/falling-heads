#!/usr/bin/env python3
"""Inspect GLB/GLTF model structure"""

import json
import struct
import sys

def inspect_glb(filepath):
    print(f"\n[INSPECT] {filepath}")
    print("=" * 80)

    with open(filepath, 'rb') as f:
        # Read GLB header
        magic = f.read(4)
        if magic != b'glTF':
            print("[ERROR] Not a valid GLB file!")
            return

        version = struct.unpack('<I', f.read(4))[0]
        length = struct.unpack('<I', f.read(4))[0]

        print(f"[OK] Valid GLB file (version {version}, {length} bytes)")

        # Read JSON chunk
        json_length = struct.unpack('<I', f.read(4))[0]
        json_type = f.read(4)
        json_data = f.read(json_length).decode('utf-8')

        gltf = json.loads(json_data)

        # Print structure
        print("\n[NODES] Bones/Objects:")
        if 'nodes' in gltf:
            for i, node in enumerate(gltf['nodes']):
                name = node.get('name', f'<unnamed_{i}>')
                children = node.get('children', [])
                mesh = node.get('mesh', None)
                skin = node.get('skin', None)

                print(f"  [{i}] {name}")
                if mesh is not None:
                    print(f"       -> Mesh: {mesh}")
                if skin is not None:
                    print(f"       -> Skin: {skin}")
                if children:
                    print(f"       -> Children: {children}")
        else:
            print("  [WARNING] No nodes found")

        # Print meshes
        print("\n[MESHES]:")
        if 'meshes' in gltf:
            for i, mesh in enumerate(gltf['meshes']):
                name = mesh.get('name', f'<unnamed_{i}>')
                primitives = len(mesh.get('primitives', []))
                print(f"  [{i}] {name} ({primitives} primitives)")
        else:
            print("  [WARNING] No meshes found")

        # Print skins (armatures/rigs)
        print("\n[SKINS/RIGS]:")
        if 'skins' in gltf:
            for i, skin in enumerate(gltf['skins']):
                name = skin.get('name', f'<unnamed_{i}>')
                joints = skin.get('joints', [])
                skeleton = skin.get('skeleton', None)
                print(f"  [{i}] {name}")
                print(f"       -> Joints/Bones: {len(joints)} bones")
                print(f"       -> Root bone: {skeleton}")

                # Print bone names
                if joints and 'nodes' in gltf:
                    print(f"       -> Bone list:")
                    for joint_idx in joints[:30]:  # Show first 30 bones
                        bone_name = gltf['nodes'][joint_idx].get('name', f'<bone_{joint_idx}>')
                        print(f"          * {bone_name}")
                    if len(joints) > 30:
                        print(f"          ... and {len(joints) - 30} more")
        else:
            print("  [WARNING] No skins/armatures found")

        # Print animations
        print("\n[ANIMATIONS]:")
        if 'animations' in gltf:
            for i, anim in enumerate(gltf['animations']):
                name = anim.get('name', f'<unnamed_{i}>')
                channels = len(anim.get('channels', []))
                samplers = len(anim.get('samplers', []))
                print(f"  [{i}] {name}")
                print(f"       -> {channels} channels, {samplers} samplers")

                # Show what bones are animated
                if 'channels' in anim and 'nodes' in gltf:
                    animated_nodes = set()
                    for channel in anim['channels']:
                        target = channel.get('target', {})
                        node_idx = target.get('node')
                        if node_idx is not None:
                            node_name = gltf['nodes'][node_idx].get('name', f'node_{node_idx}')
                            animated_nodes.add(node_name)

                    if animated_nodes:
                        print(f"       -> Animates: {', '.join(sorted(list(animated_nodes))[:10])}")
                        if len(animated_nodes) > 10:
                            print(f"          ... and {len(animated_nodes) - 10} more")
        else:
            print("  [WARNING] No animations found in model")

        # Print materials
        print("\n[MATERIALS]:")
        if 'materials' in gltf:
            for i, mat in enumerate(gltf['materials']):
                name = mat.get('name', f'<unnamed_{i}>')
                print(f"  [{i}] {name}")
        else:
            print("  [WARNING] No materials found")

        # Print scenes
        print("\n[SCENES]:")
        if 'scenes' in gltf:
            for i, scene in enumerate(gltf['scenes']):
                name = scene.get('name', f'<unnamed_{i}>')
                nodes = scene.get('nodes', [])
                print(f"  [{i}] {name} (root nodes: {nodes})")

        print("\n" + "=" * 80)
        print("[DONE] Inspection complete!\n")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python inspect_glb.py <path_to_glb_file>")
        print("\nExample:")
        print("  python inspect_glb.py assets/models/projectiles/arrow2.glb")
        sys.exit(1)

    inspect_glb(sys.argv[1])
