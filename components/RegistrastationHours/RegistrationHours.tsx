import {Button, Divider, Group, Popover, Stack, Text} from "@mantine/core";
import {SelectedTable} from "../../types/room";
import React from "react";
import {MdOutlineNoAccounts, MdVpnKey} from "react-icons/md";
import {
    GameTable,
    GuestInvite,
    MemberTypes,
    Profile,
    Reservation,
    ReservationRestriction,
    ReservationStatus
} from "../../types/wrapper";
import {useProfile} from "../ProfileProvider";
import {Database} from "../../types/database.types";
import {useSupabaseClient} from "@supabase/auth-helpers-react";

function TableButtons(
    gameTable: GameTable[],
    selectedTableId: string | null,
    startHour: number,
    selectedStartHour: number | null,
    onSelectTable: (s: SelectedTable) => void
) {
    return gameTable.map((gameTable) => {
        return (
            <Button
                variant={(gameTable.id == selectedTableId && startHour == selectedStartHour) ? "filled" : "outline"}
                key={gameTable.id}
                fullWidth={false}
                onClick={() => onSelectTable(new SelectedTable(startHour, gameTable))}>
                {gameTable.name}
            </Button>
        )
    })
}

interface IRegistrationHoursProps {
    start: number,
    end: number,
    duration: number,
}

export default function RegistrationHours(
    gameTables: GameTable[],
    selectedDateReservations: Reservation[],
    selectedRestrictions: ReservationRestriction[],
    selectedDateInvites: GuestInvite[],
    profiles: Profile[],
    selectedTable: SelectedTable | null,
    onSelectTable: (s: SelectedTable) => void,
    {start, end, duration}: IRegistrationHoursProps
) {
    const supabase = useSupabaseClient<Database>()
    const userProfile = useProfile()

    const selectedTableId = selectedTable?.table?.id || null;
    const selectedStartHour = selectedTable?.startHour || -1;

    let lastIndex = 0;
    let content: JSX.Element[] = [];

    for (let hour = start; hour < end; hour += duration) {
        const restriction = selectedRestrictions.find(value => value.start_hour == hour)

        content.push(<Stack key={hour}>
            <Group noWrap={true} style={{marginLeft: "1em", marginRight: "1em"}} spacing={'lg'}>
                <Text>{`Ora ${hour} - ${hour + duration}`}:</Text>

                {!restriction ? (
                    TableButtons(gameTables, selectedTableId, hour, selectedStartHour, onSelectTable)
                ) : (
                    <Text color={'red'} size={'lg'}>{restriction.message}</Text>
                )}
            </Group>

            {!restriction &&
                <Group style={{marginLeft: "1em", marginRight: "1em"}} spacing={"xs"}>
                    <Text>Listă înscriși: </Text>
                    {selectedDateReservations.filter(value => value.start_hour == hour).map((reservation, index) => {
                        lastIndex = index;
                        const profile = profiles.find(value => value.id == reservation.user_id)

                        if (!profile)
                            return <></>

                        const icon = profile.has_key ? <MdVpnKey/> : <></>;
                        const buttonColor = profile.role == MemberTypes.Antrenor ? 'orange' : (profile.has_key ? 'blue' : 'gray');

                        return <Popover width={200} withArrow={true} shadow={"md"} key={profile.id}>
                            <Popover.Target>
                                <Button color={buttonColor} radius={'xl'}
                                        size={'xs'} rightIcon={icon}>{index + 1}. {profile.name}</Button>
                            </Popover.Target>

                            <Popover.Dropdown>
                                <Stack align={'center'}>
                                    <Text size="sm">Creat
                                        pe {new Date(reservation.created_at).toLocaleDateString("ro-RO")}</Text>

                                    {(userProfile.profile?.role === MemberTypes.Fondator || reservation.user_id === userProfile.profile?.id) &&

                                        <Button onClick={async () => {
                                            const newData = {
                                                ...reservation,
                                                status: ReservationStatus.Cancelled
                                            }

                                            await supabase.from('rezervari').update(newData)
                                        }
                                        }>Anulează</Button>

                                    }
                                </Stack>
                            </Popover.Dropdown>
                        </Popover>

                    })}

                    {selectedDateInvites.filter(value => value.start_hour == hour).map((invite, index) => {
                        return <Button
                            key={invite.start_date + invite.start_hour + invite.guest_name} color={invite.special ? 'pink' : 'cyan'} radius={'xl'}
                            size={'xs'} rightIcon={<MdOutlineNoAccounts/>}>
                            {lastIndex + index + 2}. {invite.guest_name}
                        </Button>
                    })}
                </Group>
            }

            <Divider variant={"dashed"}/>
        </Stack>);
    }

    return content;
}
