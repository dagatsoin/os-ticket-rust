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
