import { streamRoom } from '../api/roomStream';
import { loadPendingRoomAction, loadRoom, sendRoomAction } from '../api/rooms';
import type { RoomPorts } from '../features/rooms/model/RoomController';
export const roomPorts: RoomPorts = {
  load: loadRoom,
  pending: loadPendingRoomAction,
  act: sendRoomAction,
  stream: streamRoom,
};
