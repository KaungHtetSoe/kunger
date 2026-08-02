import { RouterProvider } from "react-router-dom";

import { QueryProvider } from "@/app/QueryProvider";
import { router } from "@/app/router";
import { NotificationProvider } from "@/components/NotificationProvider";

function App() {
  return (
    <QueryProvider>
      <NotificationProvider>
        <RouterProvider router={router} />
      </NotificationProvider>
    </QueryProvider>
  );
}

export default App;
