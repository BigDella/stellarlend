import { NotificationMessage } from '../../../types/notifications';
import logger from '../../../utils/logger';

class DiscordChannel {
  async send(message: NotificationMessage): Promise<boolean> {
    try {
      logger.info('Sending Discord notification', { 
        id: message.id, 
        title: message.title,
        userId: message.userId 
      });
      // TODO: Integrate with Discord Webhook API
      return true;
    } catch (error) {
      logger.error('Discord channel failed', { error, messageId: message.id });
      return false;
    }
  }
}

export const discordChannel = new DiscordChannel();
