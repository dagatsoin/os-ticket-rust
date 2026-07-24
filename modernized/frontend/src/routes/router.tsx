// ONE react-router with THREE branches under a shared AppShell (ROADMAP Decisions → 6):
//   - public:        "/" (home) + "/open" (open-ticket form)
//   - client portal: "/tickets/*"
//   - staff:         "/staff/*"
// Feature tickets (B3 / C4 / D2) replace the placeholder elements below.
import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "../components/AppShell";
import { PublicHomePage } from "../pages/PublicHomePage";
import { OpenTicketPage } from "../pages/OpenTicketPage";
import {
  StaffLoginPage,
  StaffDashboardPage,
  StaffQueuePage,
  StaffTicketDetailPage,
} from "../pages/StaffArea";
import { ClientLoginPage, ClientTicketsPage } from "../pages/ClientPortal";
import { NotFoundPage } from "../pages/NotFoundPage";
import { AdminLayout } from "../pages/admin/AdminLayout";
import { AdminIndex, AdminPlaceholder } from "../pages/admin/AdminMisc";
import { AttachmentsSettingsPage } from "../pages/admin/AttachmentsSettingsPage";
import { StaffListPage } from "../pages/admin/StaffListPage";
import { GroupListPage } from "../pages/admin/GroupListPage";
import { SlaListPage } from "../pages/admin/SlaListPage";
import { PrioritiesPage } from "../pages/admin/PrioritiesPage";
import { PageListPage } from "../pages/admin/PageListPage";
import { LogViewerPage } from "../pages/admin/LogViewerPage";
import { CannedListPage } from "../pages/admin/CannedListPage";
import { DeptListPage } from "../pages/admin/DeptListPage";
import { TeamListPage } from "../pages/admin/TeamListPage";
import { HelpTopicListPage } from "../pages/admin/HelpTopicListPage";
import { FaqCategoryListPage } from "../pages/admin/FaqCategoryListPage";
import { ProfilePage } from "../pages/staff/ProfilePage";
import { DirectoryPage } from "../pages/staff/DirectoryPage";
import { RequireAdmin, RequireAdminArea, RequireCapability, RequireStaff } from "./guards";

export function AppRoutes() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        {/* Public branch */}
        <Route index element={<PublicHomePage />} />
        <Route path="open" element={<OpenTicketPage />} />

        {/* Staff branch — /staff/* */}
        <Route path="staff">
          <Route index element={<StaffDashboardPage />} />
          <Route path="login" element={<StaffLoginPage />} />
          <Route path="tickets" element={<StaffQueuePage />} />
          <Route path="tickets/:id" element={<StaffTicketDetailPage />} />

          {/* Non-admin staff screens (TS-M4-B6): own profile + directory. Any
              authenticated staff member may reach these — RequireStaff, NOT the
              admin gate. Live outside the AdminLayout shell. */}
          <Route
            path="profile"
            element={
              <RequireStaff>
                <ProfilePage />
              </RequireStaff>
            }
          />
          <Route
            path="directory"
            element={
              <RequireStaff>
                <DirectoryPage />
              </RequireStaff>
            }
          />

          {/* Admin panel branch — /staff/admin/* (TS-M4-A0). RequireAdminArea gates
              the shared shell (admin OR any delegated capability); admin-only bodies
              add a strict RequireAdmin. Later M4 epics add nav entries + routes here. */}
          <Route
            path="admin"
            element={
              <RequireAdminArea>
                <AdminLayout />
              </RequireAdminArea>
            }
          >
            <Route index element={<AdminIndex />} />
            <Route
              path="settings/attachments"
              element={
                <RequireAdmin>
                  <AttachmentsSettingsPage />
                </RequireAdmin>
              }
            />
            {/* Staff + Groups management (TS-M4-B2 / B4) — strict admin. */}
            <Route
              path="staff"
              element={
                <RequireAdmin>
                  <StaffListPage />
                </RequireAdmin>
              }
            />
            <Route
              path="groups"
              element={
                <RequireAdmin>
                  <GroupListPage />
                </RequireAdmin>
              }
            />
            {/* Departments, Teams & Help Topics (EPIC-M4-C) — strict admin. */}
            <Route
              path="departments"
              element={
                <RequireAdmin>
                  <DeptListPage />
                </RequireAdmin>
              }
            />
            <Route
              path="teams"
              element={
                <RequireAdmin>
                  <TeamListPage />
                </RequireAdmin>
              }
            />
            <Route
              path="help-topics"
              element={
                <RequireAdmin>
                  <HelpTopicListPage />
                </RequireAdmin>
              }
            />
            {/* FAQ Categories (EPIC-M4-E) — delegated: can_manage_faq OR admin. */}
            <Route
              path="faq-categories"
              element={
                <RequireCapability flag="can_manage_faq">
                  <FaqCategoryListPage />
                </RequireCapability>
              }
            />
            {/* SLA plans + read-only priorities (EPIC-M4-D) — strict admin. */}
            <Route
              path="sla"
              element={
                <RequireAdmin>
                  <SlaListPage />
                </RequireAdmin>
              }
            />
            <Route
              path="priorities"
              element={
                <RequireAdmin>
                  <PrioritiesPage />
                </RequireAdmin>
              }
            />
            {/* Site pages (EPIC-M4-F) — strict admin. */}
            <Route
              path="pages"
              element={
                <RequireAdmin>
                  <PageListPage />
                </RequireAdmin>
              }
            />
            {/* System logs (EPIC-M4-G) — strict admin. */}
            <Route
              path="logs"
              element={
                <RequireAdmin>
                  <LogViewerPage />
                </RequireAdmin>
              }
            />
            {/* Canned responses (EPIC-M4-H) — delegated: can_manage_premade OR admin. */}
            <Route
              path="canned"
              element={
                <RequireCapability flag="can_manage_premade">
                  <CannedListPage />
                </RequireCapability>
              }
            />
            {/* Nav entries whose screens land in later epics — no blank outlet. */}
            <Route path="*" element={<AdminPlaceholder />} />
          </Route>
        </Route>

        {/* Client portal branch — /tickets/* */}
        <Route path="tickets">
          <Route index element={<ClientTicketsPage />} />
          <Route path="login" element={<ClientLoginPage />} />
        </Route>

        {/* Legacy/alias redirect + catch-all */}
        <Route path="home" element={<Navigate to="/" replace />} />
        <Route path="*" element={<NotFoundPage />} />
      </Route>
    </Routes>
  );
}
