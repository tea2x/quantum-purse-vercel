import React from "react";
import { useSelector } from "react-redux";
import { Navigate, Outlet } from "react-router-dom";
import { RootState } from "../store";
import { ROUTES } from "../utils/constants";
import Layout from "./Layout";
import styles from "./Layout.module.scss";
type AuthLayoutProps = React.HTMLAttributes<HTMLDivElement>;

const InactiveLayout: React.FC<AuthLayoutProps> = ({ ...rest }) => {
  const { loading, wallet } = useSelector((state: RootState) => state);
  const { global: loadingGlobal } = loading;

  if (wallet.current.address && !loadingGlobal) {
    return <Navigate to={ROUTES.WALLET} />;
  }

  return (
    <Layout className={styles.inactiveLayout} {...rest}>
      <Outlet />
    </Layout>
  );
};

export default InactiveLayout;
