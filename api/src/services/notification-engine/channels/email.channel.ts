import { NotificationMessage } from '../../../types/notifications';
import logger from '../../../utils/logger';

class EmailChannel {
  async send(message: NotificationMessage): Promise<boolean> {
    try {
      logger.info('Sending email notification', { 
        id: message.id, 
        title: message.title,
        userId: message.userId 
      });
      // TODO: Integrate with email provider (SendGrid, SES, etc.)
      return true;
    } catch (error) {
      logger.error('Email channel failed', { error, messageId: message.id });
      return false;
    }
  }
}

export const emailChannel = new EmailChannel();
