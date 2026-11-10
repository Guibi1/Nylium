# Vendored Steel core

This is `steel-core` from SteelMC revision `fb63e053ea5ecb26dc9faaa3a80e19fb20d7e079`, licensed under AGPL-3.0-or-later (see LICENSE). Its manifest resolves the original workspace dependencies directly; its internal dependencies use the same SteelMC revision. Tests and benchmarks from SteelMC are omitted.

Nylium changes `World::cleanup` and `Server::save_and_shutdown` to collect and return save errors while continuing the remaining shutdown saves. When updating Steel, carry these changes forward or remove this copy once upstream exposes equivalent error handling.
