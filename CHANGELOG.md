# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-06

### Added
- Initial release of cdpkit
- Type-safe Chrome DevTools Protocol (CDP) client
- Full async/await support with tokio
- Stream-based event handling
- Auto-generated bindings from official CDP specification
- Automatic WebSocket URL discovery from Chrome debugging port
- 7 working examples (basic, auto_connect, evaluate, dom, events, network, screenshot)
- Support for all CDP domains (page, network, runtime, dom, etc.)
- Comprehensive documentation in English and Chinese

### Features
- Direct protocol access without abstraction layers
- Compile-time type safety for all CDP operations
- Flexible connection methods (host:port or WebSocket URL)
- Lightweight with minimal dependencies
- Builder pattern for all CDP commands

[0.1.0]: https://github.com/yie1d/cdpkit-rs/releases/tag/v0.1.0
