import {Button, Group, Space, Stack, Text} from "@mantine/core";
import React, {ReactElement} from "react";
import {MdCancel} from "react-icons/md";
import {getEndHour, Location, Reservation} from "../../types/wrapper";

function ShowStatus(reservation: Reservation, onCancel: (() => Promise<void>) | null) {
    if (reservation.cancelled) {
        return <Stack align={"center"} spacing={'xs'}>
            <MdCancel size={32}/>
            <Text weight={700}>Anulată</Text></Stack>
    }

    if (onCancel != null) {
        return <Button
            gradient={{from: 'orange', to: 'red'}} variant={"outline"}
            onClick={onCancel}>Anulează</Button>
    }

    return <></>
}

export default function ReservationComponent(
    reservation: Reservation,
    location: Location,
    showStatus: boolean,
    onCancel: (() => Promise<void>) | null
): ReactElement {
    return <Group position={"apart"}>
        <Stack spacing={0}>
            <Text>Pe data de <b>{(new Date(reservation.start_date)).toLocaleDateString('ro-RO')}</b> de la
                ora <b>{reservation.start_hour}:{'00'}</b> la <b>{getEndHour(reservation, location)}:{'00'}</b></Text>

            <Space h={"xs"}/>

            <Text>Locația: <b>{reservation.location.toUpperCase()}</b></Text>
        </Stack>

        {showStatus &&
            ShowStatus(reservation, onCancel)
        }
    </Group>
}
