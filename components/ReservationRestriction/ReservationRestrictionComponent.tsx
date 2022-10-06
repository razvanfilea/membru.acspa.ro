import {ActionIcon, Group, Space, Stack, Text} from "@mantine/core";
import React from "react";
import {ReservationRestriction} from "../../types/wrapper";
import {MdDelete} from "react-icons/md";

export default function ReservationRestrictionComponent(reservationBlock: ReservationRestriction, onDelete: () => Promise<void>) {
    return (<Group position={'apart'}>
        <Stack spacing={0}>
            <Text>Data: <b>{(new Date(reservationBlock.date)).toLocaleDateString('ro-RO')}</b></Text>
            <Text>De la ora {reservationBlock.start_hour}:{'00'}</Text>

            <Space h={"xs"}/>

            <Text>Motiv: <b>{reservationBlock.message}</b></Text>
        </Stack>

        <ActionIcon color={'red'} variant={'filled'} onClick={async () => onDelete()}>
            <MdDelete size={64}/>
        </ActionIcon>
    </Group>)
}
