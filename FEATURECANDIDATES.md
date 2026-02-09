● Here's what I found — ranked by impact:
  #: 1
  Feature: Server properties in edit view
  Status: DONE
  ────────────────────────────────────────
  #: 2
  Feature: Modrinth browser
  Effort: Large
  Why: ModpackSource::Modrinth and Docker env generation exist, but there's no API client or search UI like CurseForge has. Users can't discover/install Modrinth
  packs.
  ────────────────────────────────────────
  #: 3
  Feature: Docker restart policy
  Effort: Small
  Why: No restart policy is set — containers don't auto-restart after a crash or host reboot. Adding unless-stopped would be a one-liner in docker/mod.rs.
  ────────────────────────────────────────
  #: 4
  Feature: FTB StoneBlock 4 placeholder
  Effort: Small
  Why: Template has pack_id: 0 with a TODO — it won't actually work. Either fill in the real ID or remove the template.
  ────────────────────────────────────────
  #: 5
  Feature: Confirmation dialog before deleting orphaned servers
  Status: DONE
  ────────────────────────────────────────
  #: 6
  Feature: Link modpack to adopted servers
  Effort: Medium
  Why: Adopting an orphaned server clears its modpack info, so it can't start properly (itzg needs TYPE/source env vars). Users need a way to
  assign a modpack template or configure the modpack source on an already-adopted server.
