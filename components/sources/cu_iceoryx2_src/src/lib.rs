use cu29::prelude::*;
use iceoryx2::node::NodeBuilder;
use iceoryx2::port::subscriber::Subscriber;
use iceoryx2::prelude::*;
use iceoryx2::service::port_factory::publish_subscribe::PortFactory;

/// This is a source task that receives messages from an iceoryx2 service.
/// P is the payload type of the messages.
pub struct IceoryxSrc<P>
where
    P: CuMsgPayload + 'static,
{
    service_name: ServiceName,
    node: iceoryx2::prelude::Node<ipc::Service>,
    service: Option<PortFactory<ipc::Service, CuMsg<P>, ()>>,
    subscriber: Option<Subscriber<ipc::Service, CuMsg<P>, ()>>,
}

impl<P> Freezable for IceoryxSrc<P> where P: CuMsgPayload {}

impl<'cl, P> CuSrcTask<'cl> for IceoryxSrc<P>
where
    P: CuMsgPayload + 'cl + 'static,
{
    type Output = output_msg!('cl, P);

    fn new(config: Option<&ComponentConfig>) -> CuResult<Self>
    where
        Self: Sized,
    {
        let config =
            config.ok_or_else(|| CuError::from("IceoryxSource: Missing configuration."))?;
        let service_name_str = config.get::<String>("service").ok_or_else(|| {
            CuError::from("IceoryxSource: Configuration requires 'service' key (string).")
        })?;

        debug!(
            "IceoryxSource: Configuring service name: {}",
            service_name_str.as_str()
        );

        let service_name = ServiceName::new(service_name_str.as_str()).map_err(|e| {
            CuError::new_with_cause("IceoryxSource: Failed to create service name.", e)
        })?;

        let node: iceoryx2::prelude::Node<ipc::Service> =
            NodeBuilder::new().create::<ipc::Service>().map_err(|e| {
                CuError::new_with_cause(
                    format!(
                        "IceoryxSource({}): Failed to create node.",
                        service_name_str
                    )
                    .as_str(),
                    e,
                )
            })?;

        Ok(Self {
            service_name,
            node,
            service: None,
            subscriber: None,
        })
    }

    fn start(&mut self, _clock: &RobotClock) -> CuResult<()> {
        let service = self
            .node
            .service_builder(&self.service_name)
            .publish_subscribe::<CuMsg<P>>()
            .open_or_create()
            .map_err(|e| {
                CuError::new_with_cause(
                    format!(
                        "IceoryxSource({}): Failed to create service.",
                        self.service_name
                    )
                    .as_str(),
                    e,
                )
            })?;

        let subscriber = service.subscriber_builder().create().map_err(|e| {
            CuError::new_with_cause(
                format!(
                    "IceoryxSource({}): Failed to create subscriber.",
                    self.service_name
                )
                .as_str(),
                e,
            )
        })?;

        self.subscriber = Some(subscriber);
        self.service = Some(service);
        Ok(())
    }

    fn process(&mut self, _clock: &RobotClock, new_msg: Self::Output) -> CuResult<()> {
        let sub = self.subscriber.as_ref().ok_or_else(|| {
            CuError::from(
                format!(
                    "IceoryxSource({}): Subscriber not found.",
                    self.service_name
                )
                .as_str(),
            )
        })?;

        if let Some(icemsg) = sub.receive().map_err(|e| {
            CuError::new_with_cause(
                format!(
                    "IceoryxSource({}): Error receiving message.",
                    self.service_name
                )
                .as_str(),
                e,
            )
        })? {
            new_msg.set_payload(
                icemsg
                    .payload()
                    .payload()
                    .ok_or(CuError::from(
                        format!(
                            "IceoryxSource({}): Failed to get payload.",
                            self.service_name
                        )
                        .as_str(),
                    ))?
                    .clone(),
            );
            new_msg.metadata.tov = icemsg.payload().metadata.tov;
        } else {
            debug!(
                "IceoryxSource({}): No message received.",
                self.service_name.as_str()
            );
        }

        Ok(())
    }
    fn stop(&mut self, _clock: &RobotClock) -> CuResult<()> {
        self.service = None;
        self.subscriber = None;
        debug!("IceoryxSource({}): Stopped.", self.service_name.as_str());
        Ok(())
    }
}
