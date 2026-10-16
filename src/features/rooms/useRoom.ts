import { useEffect, useMemo, useSyncExternalStore } from 'react';
import type { RoomConnection } from '../../api/rooms';
import { RoomController, type RoomPorts } from './model/RoomController';
export function useRoom(ports: RoomPorts, connection: RoomConnection) {
  const controller = useMemo(() => new RoomController(ports), [ports]);
  const view = useSyncExternalStore(controller.subscribe, controller.getSnapshot);
  useEffect(() => {
    void controller.activate(connection);
    return () => controller.leave();
  }, [controller, connection]);
  return { view, controller };
}
