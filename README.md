# Distributed State Machine in Rust

This project demonstrates a simplified implementation of a distributed state machine using Rust. It showcases basic node communication, state proposals, and consensus among nodes in a distributed system.

## Overview

The project simulates a network of nodes that can propose changes to their state and reach consensus through a simplified protocol. It's designed to provide a foundational understanding of distributed state machines, covering core aspects like node setup, message passing, and consensus building.

## Features

- Node communication over TCP
- Proposal broadcasting and acknowledgement handling
- Simplified consensus mechanism
- State change proposals and commits based on majority consensus

## Getting Started

### Prerequisites

- Rust programming language: Ensure you have Rust installed on your system. You can download it from [the official Rust website](https://www.rust-lang.org/learn/get-started).

### Architecture

This project consists of several key components:

Node: Represents a node in the distributed system, capable of sending, receiving, and processing messages.

Message: Defines the structure of messages exchanged between nodes, including proposals, acknowledgements, and commits.

State: Enumerates the possible states of a node.

MessageType: Enumerates the types of messages that can be sent between nodes.

### Contributing
Contributions are welcome! If you have suggestions for improvements or encounter any issues, please feel free to open an issue or submit a pull request.

### License
This project is licensed under the MIT License - see the LICENSE file for details.

### Acknowledgments
Special thanks to the Rust community for the comprehensive documentation and resources.
Inspired by consensus algorithms like Raft and Paxos, which provide the theoretical foundation for distributed state machines.
