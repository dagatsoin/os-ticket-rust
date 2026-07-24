// Declarative schema for the admin settings surface (TS-M4-A2 / TS-M4-A3).
// One source of truth shared by the AdminSettingsStore (value coercion + wire
// serialization) and the settings UI (field rendering). Field `key`s are the
// backend config keys; `tab.id` is the wire `tab` value sent to PUT
// /api/staff/admin/settings and the key under GET `tabs`.
//
// NOTE (contract reconciliation — verified LIVE against TS-M4-A1 on 2026-07-24):
// tab ids and all field `key`s below match the running backend's
// GET/PUT /api/staff/admin/settings. Reconciled key fixes applied:
//   helpdesk_name→helpdesk_title, default_topic_id→default_help_topic,
//   thank_you_page_id→thank-you_page_id, overlimit_autoresponder→overlimit_notice_active,
//   +admin_email (emails). Selects read the live `options` lists (departments,
//   sla_plans, help_topics, priorities, email_accounts, template_groups, timezones).
// KNOWN GAP: the backend GET does NOT yet return a `pages` option list, so the
// Site Pages selects (landing/offline/thank-you) have no choices to render — the
// config keys exist and round-trip, but the picker needs the backend to add an
// `options.pages: [{id,name}]` list (flagged to the backend team).
//
// @implements FS-032.3: System settings tab fields.
// @implements FS-032.4: Ticket settings & options tab fields.
// @implements FS-032.5: Email settings tab fields.
// @implements FS-032.6: Pages / Autoresponder / Knowledgebase / Alerts / Attachments fields.

/** Named option list carried by the settings GET payload. */
export type OptionSource =
  | "departments"
  | "sla_plans"
  | "help_topics"
  | "priorities"
  | "email_accounts"
  | "template_groups"
  | "timezones"
  | "pages";

export type FieldType = "text" | "number" | "select" | "checkbox";

export interface FieldDef {
  key: string;
  label: string;
  type: FieldType;
  /** For `select` fields: which option list from the GET payload populates it. */
  optionSource?: OptionSource;
  /** For `select` fields with a fixed option set (no server list). */
  staticOptions?: { id: string; label: string }[];
  /** Required selects/text: an empty submission surfaces an inline error (server-authoritative). */
  required?: boolean;
  /** Helper text under the field when there is no error. */
  helper?: string;
  /**
   * A field gated by a master checkbox on the same tab (BS-032.6): rendered
   * disabled while the referenced checkbox key is falsy.
   */
  dependsOn?: string;
}

export interface TabDef {
  id: string;
  label: string;
  fields: FieldDef[];
}

/** The seven in-panel tabs (FS-032.1) — Attachments is a SEPARATE screen, below. */
export const SETTINGS_TABS: TabDef[] = [
  {
    id: "system",
    label: "System",
    fields: [
      { key: "helpdesk_title", label: "Helpdesk Name", type: "text", required: true },
      { key: "helpdesk_url", label: "Helpdesk URL", type: "text" },
      { key: "default_timezone_id", label: "Default Timezone", type: "select", optionSource: "timezones" },
      { key: "max_page_size", label: "Default Page Size", type: "number", helper: "Rows per queue page." },
      { key: "passwd_reset_period", label: "Password Reset Period (days)", type: "number" },
      { key: "staff_session_timeout", label: "Staff Session Timeout (min)", type: "number" },
      { key: "client_session_timeout", label: "Client Session Timeout (min)", type: "number" },
    ],
  },
  {
    id: "tickets",
    label: "Ticket Settings",
    fields: [
      {
        key: "default_ticket_status",
        label: "Default Status",
        type: "select",
        staticOptions: [
          { id: "open", label: "Open" },
          { id: "closed", label: "Closed" },
        ],
      },
      { key: "default_priority_id", label: "Default Priority", type: "select", optionSource: "priorities" },
      { key: "default_dept_id", label: "Default Department", type: "select", optionSource: "departments" },
      { key: "default_sla_id", label: "Default SLA", type: "select", optionSource: "sla_plans" },
      { key: "default_help_topic", label: "Default Help Topic", type: "select", optionSource: "help_topics" },
      { key: "ticket_lock_time", label: "Lock Time (min)", type: "number" },
      { key: "max_open_tickets", label: "Max Open Tickets", type: "number" },
      { key: "enable_captcha", label: "Enable CAPTCHA on new tickets", type: "checkbox" },
    ],
  },
  {
    id: "emails",
    label: "Email",
    fields: [
      { key: "admin_email", label: "Admin Email", type: "text", required: true },
      {
        key: "default_email_id",
        label: "Default Email",
        type: "select",
        optionSource: "email_accounts",
        required: true,
      },
      {
        key: "default_template_id",
        label: "Default Template",
        type: "select",
        optionSource: "template_groups",
        required: true,
      },
    ],
  },
  {
    id: "pages",
    label: "Site Pages",
    fields: [
      { key: "landing_page_id", label: "Landing Page", type: "select", optionSource: "pages" },
      { key: "offline_page_id", label: "Offline Page", type: "select", optionSource: "pages" },
      { key: "thank-you_page_id", label: "Thank-You Page", type: "select", optionSource: "pages" },
    ],
  },
  {
    id: "kb",
    label: "Knowledgebase",
    fields: [
      { key: "enable_kb", label: "Enable Knowledgebase", type: "checkbox" },
      { key: "enable_premade", label: "Enable Canned Responses", type: "checkbox" },
    ],
  },
  {
    id: "autoresp",
    label: "Autoresponder",
    fields: [
      { key: "ticket_autoresponder", label: "New Ticket Autoresponse", type: "checkbox" },
      { key: "message_autoresponder", label: "New Message Autoresponse", type: "checkbox" },
      { key: "overlimit_notice_active", label: "Overlimit Notice", type: "checkbox" },
    ],
  },
  {
    id: "alerts",
    label: "Alerts & Notices",
    fields: [
      { key: "ticket_alert_active", label: "New Ticket Alert", type: "checkbox" },
      { key: "ticket_alert_admin", label: "Admin Email", type: "checkbox", dependsOn: "ticket_alert_active" },
      { key: "ticket_alert_dept_manager", label: "Department Manager", type: "checkbox", dependsOn: "ticket_alert_active" },
      { key: "ticket_alert_dept_members", label: "Department Members", type: "checkbox", dependsOn: "ticket_alert_active" },
      { key: "message_alert_active", label: "New Message Alert", type: "checkbox" },
    ],
  },
];

/** The standalone Attachments screen (FS-032.1: a sub-route, NOT one of the seven tabs). */
export const ATTACHMENTS_TAB: TabDef = {
  id: "attach",
  label: "Attachments",
  fields: [
    { key: "allow_attachments", label: "Allow Attachments", type: "checkbox" },
    {
      key: "allowed_filetypes",
      label: "Allowed File Types",
      type: "text",
      dependsOn: "allow_attachments",
      helper: "Comma-separated extensions, e.g. .pdf,.png,.jpg",
    },
    {
      key: "max_file_size",
      label: "Max File Size (bytes)",
      type: "number",
      dependsOn: "allow_attachments",
    },
  ],
};

/** Every tab (tabs + the attachments screen), for store-side coercion/serialization. */
export const ALL_TABS: TabDef[] = [...SETTINGS_TABS, ATTACHMENTS_TAB];

/** Look up a tab definition by its wire id. */
export function tabDef(tabId: string): TabDef | undefined {
  return ALL_TABS.find((t) => t.id === tabId);
}
