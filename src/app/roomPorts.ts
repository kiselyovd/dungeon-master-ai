import { streamRoom } from '../api/roomStream';
import {
  loadPendingRoomAction,
  loadRoom,
  loadRoomAgentRecovery,
  sendRoomAction,
} from '../api/rooms';
import type { RoomPorts } from '../features/rooms/model/RoomController';
export const roomPorts: RoomPorts = {
  recovery: (roomId, memberId) => loadRoomAgentRecovery(roomId, memberId),
  load: loadRoom,
  pending: loadPendingRoomAction,
  act: sendRoomAction,
  stream: streamRoom,
};
