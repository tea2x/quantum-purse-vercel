import { Button, Grid, Dropdown, Divider } from "antd";
import React, { useContext, useEffect } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import LayoutCtx from "../../context/layout_ctx";
import { ROUTES } from "../../utils/constants";
import { cx } from "../../utils/methods";
import Icon from "../icon/icon";
import styles from "./header.module.scss";
import { useSelector } from "react-redux";
import { RootState } from "../../store";
import { STORAGE_KEYS } from "../../utils/constants";

const { useBreakpoint } = Grid;

const PeerValue: React.FC<{ value: number }> = ({ value }) => (
  <span className={styles.blinker}>{value}</span>
);

interface HeaderProps extends React.HTMLAttributes<HTMLDivElement> {}

const Header: React.FC<HeaderProps> = ({ className, ...rest }) => {
  const syncStatus = useSelector((state: RootState) => state.wallet.syncStatus);
  const navigate = useNavigate();
  const { showSidebar, setShowSidebar } = useContext(LayoutCtx);
  const screens = useBreakpoint();

  const location = useLocation();

  useEffect(() => {
    if ("md" in screens && !screens.md) {
      setShowSidebar(false);
    }
  }, [location.pathname, screens.md]);

  return (
    <header className={cx(styles.header, className)} {...rest}>
      <div className="header-left">
        <Icon.Chip
          color="var(--black)"
          onClick={() => {
            const step = localStorage.getItem(STORAGE_KEYS.WALLET_STEP);
            if (!step) {
              navigate(ROUTES.HOME);
            }
          }}
        />
        <p className={styles.text}>Quantum Purse</p>
      </div>

      <div className="header-right">
        <Dropdown
          overlay={
            <div className={styles.syncStatusOverlay}>
              <h2>Node Id</h2>
              {syncStatus && syncStatus.nodeId}
              <br />
              <br />
              <h2>Peers Information</h2>
              Connected: <PeerValue value={syncStatus && parseInt(syncStatus.connections.toString())}/> &nbsp; &nbsp; Sync: {syncStatus && syncStatus.syncedStatus.toFixed(2)}%
            </div>
          }
          trigger={["hover"]}
        >
          <Icon.Connections className={styles.spinAndPause} />
        </Dropdown>

        <span className={styles.firstGlance}>
          {syncStatus && parseInt(syncStatus.connections.toString())}
        </span>
        
        <Dropdown
          overlay={
            <div className={styles.syncStatusOverlay}>
              <h2>Node Id</h2>
              {syncStatus && syncStatus.nodeId}
              <br />
              <br />
              <h2>Network Status</h2>
              Start: {syncStatus && syncStatus.startBlock.toLocaleString()} &nbsp; &nbsp; Synced: {syncStatus && syncStatus.syncedBlock.toLocaleString()} &nbsp; &nbsp; Tip: {syncStatus && syncStatus.tipBlock.toLocaleString()}
            </div>
          }
          trigger={["hover"]}
        >
          <Icon.Syncing className={styles.spinHarmonic}/>
        </Dropdown>

        <span className={styles.firstGlance}>
          {syncStatus && syncStatus.syncedStatus.toFixed(2)}%
        </span>
        
        {!screens.md && (
          <Button
            type="text"
            onClick={() => setShowSidebar(!showSidebar)}
            icon={<Icon.Hamburger color="var(--white)" />}
          />
        )}
      </div>

    </header>
  );
};

export default Header;
