import {Group, Button, Loader, Space, Stack, Text} from "@mantine/core";
import React from "react";
import {MdCancel, MdDone, MdErrorOutline} from "react-icons/md";
import {GameTable, getEndDate, getStartDate, Reservation, ReservationStatus} from "../../types/wrapper";

interface Status {
    icon: React.ReactNode,
    message: string
}

function cancelReservation(reservation: Reservation) {
    // TODO
}

export default function ReservationComponent(reservation, gameTable: GameTable, showStatus: boolean) {
    // const theme = useMantineTheme()

    const state = reservation.status ?? ReservationStatus.PendingApproval;
    const status: Status = (() => {
        switch (state) {
            case ReservationStatus.PendingApproval:
                return {icon: <Loader size={32}/>, message: "Se procesează"};
            case ReservationStatus.Canceled:
                return {icon: <MdCancel size={32}/>, message: "Anulata"};
            case ReservationStatus.Approved:
                return {icon: <MdDone size={32}/>, message: "Aprobată"};
            case ReservationStatus.Invalid:
                return {icon: <MdErrorOutline size={32}/>, message: "Eroare la aprobare"}
        }
    })();

    const startDate = getStartDate(reservation);
    const endDate = getEndDate(reservation);

    return (<Group position={"apart"}>
        <Stack spacing={0}>
            <Text size={"lg"} weight={800}>{gameTable.name}</Text>

            <Text>Pe data de <b>{startDate.toLocaleDateString('ro-RO')}</b> de la ora <b>{startDate.getHours()}:{("0" + startDate.getMinutes()).slice(-2)}</b> la <b>{endDate.getHours()}:{("0" + endDate.getMinutes()).slice(-2)}</b></Text>

            <Space h={"xs"}/>

            <Text>Tipul mesei: <b>{gameTable.type.toUpperCase()}</b></Text>
            <Text>Culoarea mesei: <b>{gameTable.color.toUpperCase()}</b></Text>
            <Text>Robot: <b>{gameTable.has_robot ? "DA" : "NU"}</b></Text>
        </Stack>

        {showStatus &&
            <Stack align={"center"}>
                {status.icon}
                <Text weight={700}>{status.message}</Text>

                <Space h={"xs"} />

                {reservation.status == ReservationStatus.Approved &&
                    <Button gradient={{ from: 'orange', to: 'red' }} variant={"outline"} onClick={() => cancelReservation(reservation)}>Anulează</Button>
                }
            </Stack>
        }
    </Group>)
}
